use identus_apollo::jwk::Jwk;
use serde::{Deserialize, Serialize};

use crate::{Did, Uri};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(
    feature = "ts-types",
    derive(ts_rs::TS),
    ts(export_to = "../../../bindings/ts-types/did_core_types.ts")
)]
#[serde(rename_all = "camelCase")]
pub struct DidDocument {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: Did,
    pub also_known_as: Option<Vec<Uri>>,
    pub verification_method: Vec<VerificationMethod>,
    pub authentication: Option<Vec<VerificationMethodOrRef>>,
    pub assertion_method: Option<Vec<VerificationMethodOrRef>>,
    pub key_agreement: Option<Vec<VerificationMethodOrRef>>,
    pub capability_invocation: Option<Vec<VerificationMethodOrRef>>,
    pub capability_delegation: Option<Vec<VerificationMethodOrRef>>,
    pub service: Option<Vec<Service>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(
    feature = "ts-types",
    derive(ts_rs::TS),
    ts(export_to = "../../../bindings/ts-types/did_core_types.ts")
)]
#[serde(rename_all = "camelCase")]
pub struct VerificationMethod {
    pub id: String,
    pub r#type: String,
    pub controller: String,
    #[cfg_attr(feature = "ts-types", ts(type = "Record<string, any> | null"))]
    pub public_key_jwk: Option<Jwk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(
    feature = "ts-types",
    derive(ts_rs::TS),
    ts(export_to = "../../../bindings/ts-types/did_core_types.ts")
)]
#[serde(untagged)]
pub enum VerificationMethodOrRef {
    Embedded(VerificationMethod),
    Ref(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(
    feature = "ts-types",
    derive(ts_rs::TS),
    ts(export_to = "../../../bindings/ts-types/did_core_types.ts")
)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub id: String,
    pub r#type: ServiceType,
    pub service_endpoint: ServiceEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(
    feature = "ts-types",
    derive(ts_rs::TS),
    ts(export_to = "../../../bindings/ts-types/did_core_types.ts")
)]
#[serde(untagged)]
pub enum ServiceType {
    Str(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(
    feature = "ts-types",
    derive(ts_rs::TS),
    ts(export_to = "../../../bindings/ts-types/did_core_types.ts")
)]
#[serde(untagged)]
pub enum ServiceEndpoint {
    StrOrMap(StringOrMap),
    List(Vec<StringOrMap>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(
    feature = "ts-types",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../bindings/ts-types/did_core_types.ts")
)]
#[serde(untagged)]
pub enum StringOrMap {
    Str(String),
    #[cfg_attr(feature = "ts-types", ts(type = "Record<string, any>"))]
    Map(serde_json::Map<String, serde_json::Value>),
}

#[cfg(test)]
#[cfg(feature = "ts-types")]
mod ts_export {
    use ts_rs::TS;

    use super::*;
    #[test]
    fn export_types() {
        DidDocument::export_all().unwrap();
    }
}
