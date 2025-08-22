use identus_did_midnight::did_doc::{DidDocument, Service, VerificationMethod};
use ts_rs::TS;

#[test]
fn export_typescript_bindings() {
    DidDocument::export().expect("Failed to export DidDocument");
    VerificationMethod::export().expect("Failed to export VerificationMethod");
    Service::export().expect("Failed to export Service");
}
