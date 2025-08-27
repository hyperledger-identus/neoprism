use std::io;
use std::process::Command;

use derive_more::{Display, Error};
use identus_did_core::DidDocument;
use identus_did_midnight::did::MidnightDid;
use identus_did_midnight::dlt::{ContractState, ContractStateDecoder};

const CLI_PATH: &str = "did-midnight-serde";

#[derive(Debug, Display, Error)]
pub enum SerdeCliError {
    #[display("cli executable not found: {source}")]
    CliNotFound { source: io::Error },
    #[display("cli returned non-zero exit code {code}")]
    NonZeroExit { code: i32 },
    #[display("cli output is empty")]
    EmptyOutput,
    #[display("cli output is not valid json: {source}")]
    InvalidJson { source: serde_json::Error },
    #[display("argument conversion failed: {msg}")]
    ArgumentConversion { msg: String },
    #[display("cli invocation failed: {source}")]
    InvocationFailed { source: io::Error },
}

pub struct CliContractStateDecoder;

impl ContractStateDecoder for CliContractStateDecoder {
    fn decode(&self, did: &MidnightDid, state: ContractState) -> Result<DidDocument, Box<dyn std::error::Error>> {
        let did_doc = decode_contract_state_via_cli(did, &state)?;
        Ok(did_doc)
    }
}

fn decode_contract_state_via_cli(
    did: &MidnightDid,
    contract_state: &ContractState,
) -> Result<DidDocument, SerdeCliError> {
    let network_id_num = did.network().as_u8_repr().to_string();
    let contract_state_hex = contract_state.inner().to_string();
    let args = [&did.to_string(), &network_id_num, &contract_state_hex];

    let output = Command::new(CLI_PATH)
        .args(args)
        .output()
        .map_err(|e| SerdeCliError::InvocationFailed { source: e })?;
    if !output.status.success() {
        return Err(SerdeCliError::NonZeroExit {
            code: output.status.code().unwrap_or(-1),
        });
    }
    let stdout = String::from_utf8(output.stdout).map_err(|_e| SerdeCliError::EmptyOutput)?;
    if stdout.trim().is_empty() {
        return Err(SerdeCliError::EmptyOutput);
    }
    let did_doc: DidDocument = serde_json::from_str(&stdout).map_err(|e| SerdeCliError::InvalidJson { source: e })?;
    Ok(did_doc)
}
