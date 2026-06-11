use std::path::PathBuf;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use derive_more::Display;
use identus_apollo::hex::HexStr;
use identus_did_prism::dlt::TxId;
use identus_did_prism::prelude::SignedPrismOperation;
use identus_did_prism::proto::MessageExt;
use identus_did_prism::proto::prism::{PrismBlock, PrismObject};
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio::time::{sleep, timeout};

use crate::DltSink;

type StdError = Box<dyn std::error::Error + Send + Sync>;

const SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RETRIES: usize = 5;
const RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Display, derive_more::Error)]
pub enum Error {
    #[display("failed to spawn embedded-wallet subprocess: {source}")]
    SubprocessSpawn { source: StdError },
    #[display("failed to write to subprocess stdin: {source}")]
    StdinWrite { source: StdError },
    #[display("failed to wait for embedded-wallet subprocess: {source}")]
    SubprocessIo { source: StdError },
    #[display("embedded-wallet subprocess failed: {stderr}")]
    SubprocessFailed { stderr: String },
    #[display("embedded-wallet subprocess timed out after 30s")]
    SubprocessTimeout,
    #[display("failed to decode CBOR hex from subprocess: {source}")]
    CborDecode { source: StdError },
    #[display("failed to submit transaction to cardano-submit-api: {source}")]
    SubmitFailed { source: StdError },
    #[display("cardano-submit-api returned non-success status {status}: {body}")]
    SubmitApiError { status: u16, body: String },
    #[display("failed to parse transaction hash from response: {source}")]
    TxHashParse { source: StdError },
}

#[derive(Debug, Clone, Copy)]
pub enum Network {
    Mainnet,
    Preprod,
    Preview,
    Custom,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Preprod => write!(f, "preprod"),
            Network::Preview => write!(f, "preview"),
            Network::Custom => write!(f, "custom"),
        }
    }
}

pub struct EmbeddedWalletSinkConfig {
    pub embedded_wallet_bin: PathBuf,
    pub blockfrost_url: String,
    pub blockfrost_api_key: Option<String>,
    pub network: Network,
    pub submit_api_url: Option<String>,
    pub mnemonic: Arc<str>,
}

pub struct EmbeddedWalletSink {
    config: EmbeddedWalletSinkConfig,
    client: Client,
    semaphore: Semaphore,
}

impl EmbeddedWalletSink {
    pub fn new(config: EmbeddedWalletSinkConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            // Semaphore limits concurrency to 1 because all transactions share a single
            // UTXO set — concurrent submissions would encounter UTXO contention
            // (double-spending the same inputs). Serializing submissions avoids this.
            semaphore: Semaphore::new(1),
        }
    }

    /// Check if the submit-api error is transient (e.g., UTXO not yet propagated)
    fn is_retryable_error(body: &str) -> bool {
        body.contains("BadInputsUTxO") || body.contains("ValueNotConservedUTxO")
    }
}

#[async_trait::async_trait]
impl DltSink for EmbeddedWalletSink {
    async fn publish_operations(&self, operations: Vec<SignedPrismOperation>) -> Result<TxId, String> {
        let prism_object = PrismObject {
            block_content: Some(PrismBlock {
                operations,
                special_fields: Default::default(),
            })
            .into(),
            special_fields: Default::default(),
        };

        // Encode PrismObject to hex string
        let prism_object_bytes = prism_object.encode_to_vec();
        let prism_object_hex = HexStr::from(&prism_object_bytes).to_string();

        let mut attempt = 0;
        loop {
            let result = {
                let _permit = self.semaphore.acquire().await.map_err(|e| e.to_string())?;
                self.build_and_submit(&prism_object_hex).await
            };

            match result {
                Ok(tx_id) => return Ok(tx_id),
                Err(e) => {
                    if !Self::is_retryable_error(&e) || attempt >= MAX_RETRIES {
                        return Err(e);
                    }

                    tracing::warn!(
                        attempt = attempt + 1,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        "transient submit error, retrying in {:?}",
                        RETRY_DELAY
                    );
                    sleep(RETRY_DELAY).await;
                    attempt += 1;
                }
            }
        }
    }
}

impl EmbeddedWalletSink {
    /// Build transaction via embedded-wallet subprocess and submit to cardano-submit-api
    async fn build_and_submit(&self, prism_object_hex: &str) -> Result<TxId, String> {
        let mut args = vec![
            "build".to_string(),
            "--network".to_string(),
            self.config.network.to_string(),
            "--prism-object-hex".to_string(),
            prism_object_hex.to_string(),
            "--mnemonic-stdin".to_string(),
        ];

        // Add either URL or API key, not both
        if let Some(api_key) = &self.config.blockfrost_api_key {
            args.push("--blockfrost-api-key".to_string());
            args.push(api_key.clone());
        } else {
            args.push("--blockfrost-url".to_string());
            args.push(self.config.blockfrost_url.clone());
        }

        let mut child = Command::new(&self.config.embedded_wallet_bin)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::SubprocessSpawn { source: e.into() }.to_string())?;

        // Write mnemonic to stdin. A broken-pipe error here means the
        // subprocess has already exited (e.g., it failed fast); fall through
        // to read the output so the caller sees the real exit status / stderr
        // rather than the stdin write error.
        {
            use std::io::ErrorKind;
            if let Some(stdin) = child.stdin.as_mut() {
                if let Err(e) = stdin.write_all(self.config.mnemonic.as_bytes()).await
                    && e.kind() != ErrorKind::BrokenPipe
                {
                    return Err(Error::StdinWrite { source: e.into() }.to_string());
                }
                if let Err(e) = stdin.write_all(b"\n").await
                    && e.kind() != ErrorKind::BrokenPipe
                {
                    return Err(Error::StdinWrite { source: e.into() }.to_string());
                }
                // Close stdin to signal EOF to the subprocess.
                drop(child.stdin.take());
            } else {
                return Err(Error::StdinWrite {
                    source: "failed to get stdin handle".into(),
                }
                .to_string());
            }
        }

        let output = timeout(SUBPROCESS_TIMEOUT, child.wait_with_output())
            .await
            .map_err(|_| Error::SubprocessTimeout.to_string())?
            .map_err(|e| Error::SubprocessIo { source: e.into() }.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(Error::SubprocessFailed { stderr }.to_string());
        }

        let cbor_hex =
            String::from_utf8(output.stdout).map_err(|e| Error::CborDecode { source: e.into() }.to_string())?;
        let cbor_hex = cbor_hex.trim();

        let cbor_bytes = HexStr::from_str(cbor_hex)
            .map_err(|e| Error::CborDecode { source: e.into() }.to_string())?
            .to_bytes();

        let resp = if let Some(submit_api_url) = &self.config.submit_api_url {
            self.client
                .post(format!("{}/api/submit/tx", submit_api_url))
                .header("Content-Type", "application/cbor")
                .body(cbor_bytes)
                .send()
                .await
                .map_err(|e| Error::SubmitFailed { source: e.into() }.to_string())?
        } else {
            let api_key = self.config.blockfrost_api_key.as_ref().ok_or_else(|| {
                Error::SubmitFailed {
                    source: "blockfrost api key required when submit-api-url is not provided".into(),
                }
                .to_string()
            })?;

            self.client
                .post(format!("{}/tx/submit", self.config.blockfrost_url))
                .header("Content-Type", "application/cbor")
                .header("project_id", api_key)
                .body(cbor_bytes)
                .send()
                .await
                .map_err(|e| Error::SubmitFailed { source: e.into() }.to_string())?
        };

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "unable to read response body".to_string());
            return Err(Error::SubmitApiError { status, body }.to_string());
        }

        let tx_hash_hex = resp
            .text()
            .await
            .map_err(|e| Error::TxHashParse { source: e.into() }.to_string())?;
        // cardano-submit-api returns JSON-quoted hex string, strip quotes if present
        let tx_hash_hex = tx_hash_hex.trim().trim_matches('"');

        let tx_hash_bytes = HexStr::from_str(tx_hash_hex)
            .map_err(|e| Error::TxHashParse { source: e.into() }.to_string())?
            .to_bytes();

        TxId::from_bytes(&tx_hash_bytes).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    /// A valid 32-byte hex string used for TxId in tests
    const VALID_TX_HASH: &str = "aabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd";

    // ------------------------------------------------------------------
    // Test helpers
    // ------------------------------------------------------------------

    /// Create a fake wallet script in the given temp dir.
    /// The script consumes stdin (mnemonic) and prints `stdout_content` to stdout.
    fn create_fake_wallet(dir: &tempfile::TempDir, stdout_content: &str) -> PathBuf {
        let path = dir.path().join("fake-wallet");
        std::fs::write(&path, format!("#!/bin/sh\ncat > /dev/null\necho '{stdout_content}'\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Create a fake wallet script that exits with non-zero and prints to stderr.
    fn create_failing_wallet(dir: &tempfile::TempDir, stderr_msg: &str) -> PathBuf {
        let path = dir.path().join("fake-wallet-fail");
        std::fs::write(&path, format!("#!/bin/sh\necho '{stderr_msg}' >&2\nexit 1\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Build a config with the given binary (submit_api_url = None, no api key).
    fn config_with_bin(bin: PathBuf) -> EmbeddedWalletSinkConfig {
        EmbeddedWalletSinkConfig {
            embedded_wallet_bin: bin,
            blockfrost_url: String::new(),
            blockfrost_api_key: None,
            network: Network::Mainnet,
            submit_api_url: None,
            mnemonic: test_mnemonic(),
        }
    }

    /// Build a config that uses submit-api at the given URL.
    fn config_with_submit_api(bin: PathBuf, submit_url: String) -> EmbeddedWalletSinkConfig {
        EmbeddedWalletSinkConfig {
            embedded_wallet_bin: bin,
            blockfrost_url: String::new(),
            blockfrost_api_key: Some("test-api-key".to_string()),
            network: Network::Mainnet,
            submit_api_url: Some(submit_url),
            mnemonic: test_mnemonic(),
        }
    }

    /// Build a config that uses blockfrost (no submit-api-url).
    fn config_with_blockfrost(
        bin: PathBuf,
        blockfrost_url: String,
        api_key: Option<String>,
    ) -> EmbeddedWalletSinkConfig {
        EmbeddedWalletSinkConfig {
            embedded_wallet_bin: bin,
            blockfrost_url,
            blockfrost_api_key: api_key,
            network: Network::Preprod,
            submit_api_url: None,
            mnemonic: test_mnemonic(),
        }
    }

    fn test_mnemonic() -> Arc<str> {
        Arc::from("test word ".repeat(4))
    }

    /// Start a mock HTTP server that accepts one connection and returns a canned response.
    async fn start_mock_server(status: u16, body: String) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}");

        let handle = tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                // Read the full request (headers + body)
                let mut buf = vec![0u8; 16384];
                let _ = stream.read(&mut buf).await;

                let status_text = if (200..300).contains(&status) { "OK" } else { "Error" };
                let response = format!(
                    "HTTP/1.1 {status} {status_text}\r\n\
                     Content-Length: {}\r\n\
                     Content-Type: application/json\r\n\
                     \r\n\
                     {body}",
                    body.len(),
                );
                let _ = stream.write_all(response.as_bytes()).await;
            }
        });

        (url, handle)
    }

    // ------------------------------------------------------------------
    // Error Display variants
    // ------------------------------------------------------------------

    #[test]
    fn error_display() {
        // SubprocessSpawn
        let msg = Error::SubprocessSpawn {
            source: "spawn failed".into(),
        }
        .to_string();
        assert!(msg.contains("embedded-wallet subprocess"), "{msg}");
        assert!(msg.contains("spawn failed"), "{msg}");

        // StdinWrite
        let msg = Error::StdinWrite {
            source: "write failed".into(),
        }
        .to_string();
        assert!(msg.contains("subprocess stdin"), "{msg}");
        assert!(msg.contains("write failed"), "{msg}");

        // SubprocessIo
        let msg = Error::SubprocessIo {
            source: "io failed".into(),
        }
        .to_string();
        assert!(msg.contains("wait for embedded-wallet subprocess"), "{msg}");
        assert!(msg.contains("io failed"), "{msg}");

        // SubprocessFailed
        let msg = Error::SubprocessFailed {
            stderr: "exit code 1".into(),
        }
        .to_string();
        assert!(msg.contains("subprocess failed"), "{msg}");
        assert!(msg.contains("exit code 1"), "{msg}");

        // SubprocessTimeout
        assert_eq!(
            Error::SubprocessTimeout.to_string(),
            "embedded-wallet subprocess timed out after 30s",
        );

        // CborDecode
        let msg = Error::CborDecode {
            source: "invalid hex".into(),
        }
        .to_string();
        assert!(msg.contains("decode CBOR"), "{msg}");
        assert!(msg.contains("invalid hex"), "{msg}");

        // SubmitFailed
        let msg = Error::SubmitFailed {
            source: "connection refused".into(),
        }
        .to_string();
        assert!(msg.contains("submit transaction"), "{msg}");
        assert!(msg.contains("connection refused"), "{msg}");

        // SubmitApiError
        let msg = Error::SubmitApiError {
            status: 503,
            body: "service unavailable".into(),
        }
        .to_string();
        assert!(msg.contains("503"), "{msg}");
        assert!(msg.contains("non-success status"), "{msg}");
        assert!(msg.contains("service unavailable"), "{msg}");

        // TxHashParse
        let msg = Error::TxHashParse {
            source: "bad hash".into(),
        }
        .to_string();
        assert!(msg.contains("transaction hash"), "{msg}");
        assert!(msg.contains("bad hash"), "{msg}");
    }

    // ------------------------------------------------------------------
    // Network::Display
    // ------------------------------------------------------------------

    #[test]
    fn network_display() {
        assert_eq!(Network::Mainnet.to_string(), "mainnet");
        assert_eq!(Network::Preprod.to_string(), "preprod");
        assert_eq!(Network::Preview.to_string(), "preview");
        assert_eq!(Network::Custom.to_string(), "custom");
    }

    // ------------------------------------------------------------------
    // EmbeddedWalletSink::new()
    // ------------------------------------------------------------------

    fn sample_config() -> EmbeddedWalletSinkConfig {
        EmbeddedWalletSinkConfig {
            embedded_wallet_bin: PathBuf::from("/usr/local/bin/embedded-wallet"),
            blockfrost_url: "https://cardano-mainnet.blockfrost.io/api/v0".to_string(),
            blockfrost_api_key: Some("test-api-key".to_string()),
            network: Network::Mainnet,
            submit_api_url: Some("http://localhost:8090".to_string()),
            mnemonic: Arc::from("test mnemonic twelve words here for unit test"),
        }
    }

    #[test]
    fn embedded_wallet_sink_new_initializes_fields() {
        let config = sample_config();
        let sink = EmbeddedWalletSink::new(config);
        // Verify the semaphore starts with 1 permit
        let permit = sink.semaphore.try_acquire();
        assert!(permit.is_ok(), "expected semaphore to have 1 permit available");
        drop(permit);
        // Verify config is stored (check a representative field)
        assert_eq!(
            sink.config.embedded_wallet_bin,
            PathBuf::from("/usr/local/bin/embedded-wallet")
        );
        assert!(matches!(sink.config.network, Network::Mainnet));
    }

    // ------------------------------------------------------------------
    // is_retryable_error()
    // ------------------------------------------------------------------

    #[test]
    fn is_retryable_error() {
        // Retryable patterns
        assert!(EmbeddedWalletSink::is_retryable_error("BadInputsUTxO at index 0"));
        assert!(EmbeddedWalletSink::is_retryable_error("ValueNotConservedUTxO mismatch"));
        assert!(EmbeddedWalletSink::is_retryable_error(
            "BadInputsUTxO and ValueNotConservedUTxO"
        ));

        // Non-retryable
        assert!(!EmbeddedWalletSink::is_retryable_error("some unrelated error message"));
        assert!(!EmbeddedWalletSink::is_retryable_error(""));
    }

    // ------------------------------------------------------------------
    // build_and_submit — subprocess error paths
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn build_and_submit_nonexistent_binary_returns_spawn_error() {
        let sink = EmbeddedWalletSink::new(config_with_bin(PathBuf::from("/nonexistent/binary")));
        let err = sink.build_and_submit("deadbeef").await.unwrap_err();
        assert!(err.contains("spawn"), "expected spawn error, got: {err}");
    }

    #[tokio::test]
    async fn build_and_submit_subprocess_failure_returns_subprocess_failed_error() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_failing_wallet(&dir, "something went wrong");
        let sink = EmbeddedWalletSink::new(config_with_bin(bin));
        let err = sink.build_and_submit("deadbeef").await.unwrap_err();
        assert!(
            err.contains("subprocess failed"),
            "expected subprocess failed, got: {err}"
        );
        assert!(
            err.contains("something went wrong"),
            "expected stderr in error, got: {err}"
        );
    }

    #[tokio::test]
    async fn build_and_submit_invalid_hex_stdout_returns_cbor_decode_error() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_fake_wallet(&dir, "this is not hex!!!");
        let sink = EmbeddedWalletSink::new(config_with_bin(bin));
        let err = sink.build_and_submit("deadbeef").await.unwrap_err();
        assert!(err.contains("decode CBOR"), "expected CBOR decode error, got: {err}");
    }

    // ------------------------------------------------------------------
    // build_and_submit — HTTP submit-api paths
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn build_and_submit_unreachable_submit_api_returns_submit_error() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_fake_wallet(&dir, VALID_TX_HASH);
        // Port 1 is not listening — connection refused
        let sink = EmbeddedWalletSink::new(config_with_submit_api(bin, "http://127.0.0.1:1".to_string()));
        let err = sink.build_and_submit("deadbeef").await.unwrap_err();
        assert!(err.contains("submit"), "expected submit error, got: {err}");
    }

    #[tokio::test]
    async fn build_and_submit_submit_api_returns_error_status() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_fake_wallet(&dir, VALID_TX_HASH);
        let (url, _server) = start_mock_server(500, "internal server error".to_string()).await;
        let sink = EmbeddedWalletSink::new(config_with_submit_api(bin, url));
        let err = sink.build_and_submit("deadbeef").await.unwrap_err();
        assert!(
            err.contains("non-success status"),
            "expected submit api error, got: {err}"
        );
        assert!(err.contains("500"), "expected status 500, got: {err}");
    }

    #[tokio::test]
    async fn build_and_submit_submit_api_success() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_fake_wallet(&dir, VALID_TX_HASH);
        let (url, _server) = start_mock_server(200, VALID_TX_HASH.to_string()).await;
        let sink = EmbeddedWalletSink::new(config_with_submit_api(bin, url));
        let tx_id = sink.build_and_submit("deadbeef").await.unwrap();
        // Verify we got a valid TxId (32 bytes)
        assert_eq!(tx_id.to_vec().len(), 32);
    }

    #[tokio::test]
    async fn build_and_submit_submit_api_json_quoted_hash() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_fake_wallet(&dir, VALID_TX_HASH);
        // cardano-submit-api may return JSON-quoted hash
        let body = format!("\"{VALID_TX_HASH}\"");
        let (url, _server) = start_mock_server(200, body).await;
        let sink = EmbeddedWalletSink::new(config_with_submit_api(bin, url));
        let tx_id = sink.build_and_submit("deadbeef").await.unwrap();
        assert_eq!(tx_id.to_vec().len(), 32);
    }

    #[tokio::test]
    async fn build_and_submit_submit_api_invalid_tx_hash_returns_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_fake_wallet(&dir, VALID_TX_HASH);
        // Return a hash that's too short (not 32 bytes)
        let (url, _server) = start_mock_server(200, "aabb".to_string()).await;
        let sink = EmbeddedWalletSink::new(config_with_submit_api(bin, url));
        let err = sink.build_and_submit("deadbeef").await.unwrap_err();
        // TxId::from_bytes fails with hash size error for short input
        assert!(
            err.contains("invalid input size"),
            "expected hash size error, got: {err}"
        );
    }

    // ------------------------------------------------------------------
    // build_and_submit — blockfrost paths
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn build_and_submit_blockfrost_success() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_fake_wallet(&dir, VALID_TX_HASH);
        let (url, _server) = start_mock_server(200, VALID_TX_HASH.to_string()).await;
        let sink = EmbeddedWalletSink::new(config_with_blockfrost(bin, url, Some("test-key".to_string())));
        let tx_id = sink.build_and_submit("deadbeef").await.unwrap();
        assert_eq!(tx_id.to_vec().len(), 32);
    }

    #[tokio::test]
    async fn build_and_submit_blockfrost_without_api_key_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_fake_wallet(&dir, VALID_TX_HASH);
        // submit_api_url is None AND blockfrost_api_key is None
        let sink = EmbeddedWalletSink::new(config_with_blockfrost(bin, "http://127.0.0.1:1".to_string(), None));
        let err = sink.build_and_submit("deadbeef").await.unwrap_err();
        assert!(err.contains("submit"), "expected submit error, got: {err}");
    }

    // ------------------------------------------------------------------
    // publish_operations — integration via DltSink trait
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn publish_operations_non_retryable_error_returns_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_failing_wallet(&dir, "permanent failure");
        let sink = EmbeddedWalletSink::new(config_with_submit_api(bin, "http://127.0.0.1:1".to_string()));
        // SubprocessFailed error is NOT retryable, so it should return immediately
        let err = sink.publish_operations(vec![]).await.unwrap_err();
        assert!(
            err.contains("subprocess failed"),
            "expected subprocess failed, got: {err}"
        );
    }

    #[tokio::test]
    async fn publish_operations_success() {
        let dir = tempfile::tempdir().unwrap();
        let bin = create_fake_wallet(&dir, VALID_TX_HASH);
        let (url, _server) = start_mock_server(200, VALID_TX_HASH.to_string()).await;
        let sink = EmbeddedWalletSink::new(config_with_submit_api(bin, url));
        let tx_id = sink.publish_operations(vec![]).await.unwrap();
        assert_eq!(tx_id.to_vec().len(), 32);
    }
}
