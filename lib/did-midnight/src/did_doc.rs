use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(TS, Serialize, Deserialize)]
#[ts(export)]
pub struct DidDocument {
    pub id: String,
    pub verification_method: Vec<VerificationMethod>,
    pub service: Option<Vec<Service>>,
}

#[derive(TS, Serialize, Deserialize)]
#[ts(export)]
pub enum VerificationMethod {
    Ed25519 { public_key: String },
    EcdsaSecp256k1 { public_key: String },
}

#[derive(TS, Serialize, Deserialize)]
#[ts(export)]
pub struct Service {
    pub id: String,
    pub type_: String,
    pub endpoint: String,
}

#[cfg(test)]
mod tests {
    use ts_rs::TS;

    use super::*;

    #[test]
    fn export_typescript_bindings() {
        DidDocument::export().expect("Failed to export DidDocument");
        VerificationMethod::export().expect("Failed to export VerificationMethod");
        Service::export().expect("Failed to export Service");
    }
}
