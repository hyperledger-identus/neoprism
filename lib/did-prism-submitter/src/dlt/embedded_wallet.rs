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
                    if !Self::is_retryable_error(&e) || attempt >= MAX_RETRIES - 1 {
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

        {
            let stdin = child.stdin.as_mut().ok_or_else(|| {
                Error::StdinWrite {
                    source: "failed to get stdin handle".into(),
                }
                .to_string()
            })?;
            stdin
                .write_all(self.config.mnemonic.as_bytes())
                .await
                .map_err(|e| Error::StdinWrite { source: e.into() }.to_string())?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| Error::StdinWrite { source: e.into() }.to_string())?;
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
