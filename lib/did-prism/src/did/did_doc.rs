use identus_apollo::jwk::EncodeJwk;
use identus_did_core::{
    Did, DidDocument, DidDocumentMetadata, DidResolutionMetadata, ResolutionResult, Service, ServiceEndpoint,
    ServiceType, StringOrMap, VerificationMethod, VerificationMethodOrRef,
};

use crate::did::operation::KeyUsage;
use crate::did::{DidState, PrismDid, PrismDidOps, operation};

impl DidState {
    pub fn to_resolution_result(&self, did: &PrismDid) -> ResolutionResult {
        let did_document = self.to_did_document(&did.to_did());
        ResolutionResult {
            did_document: Some(did_document).filter(|_| !self.is_deactivated()),
            did_resolution_metadata: DidResolutionMetadata {
                content_type: Some("application/did-resolution".to_string()),
                ..Default::default()
            },
            did_document_metadata: DidDocumentMetadata {
                created: None, // TODO: populate this field
                updated: None, // TODO: populate this field
                deactivated: Some(self.is_deactivated()),
                canonical_id: Some(did.clone().into_canonical().to_did()),
            },
        }
    }

    pub fn to_did_document(&self, did: &Did) -> DidDocument {
        let mut context = vec!["https://www.w3.org/ns/did/v1".to_string()];
        context.extend(self.context.clone());

        let get_relationship = |usage: KeyUsage| -> Vec<VerificationMethodOrRef> {
            self.public_keys
                .iter()
                .filter(|k| k.data.usage() == usage)
                .map(|k| VerificationMethodOrRef::Ref(format!("{}#{}", did, k.id)))
                .collect()
        };
        let verification_method = self
            .public_keys
            .iter()
            .filter(|k| {
                const W3C_KEY_TYPES: [KeyUsage; 5] = [
                    KeyUsage::AuthenticationKey,
                    KeyUsage::IssuingKey,
                    KeyUsage::KeyAgreementKey,
                    KeyUsage::CapabilityInvocationKey,
                    KeyUsage::CapabilityDelegationKey,
                ];
                W3C_KEY_TYPES.iter().any(|usage| usage == &k.data.usage())
            })
            .flat_map(|k| transform_key_jwk(did, k))
            .collect();
        DidDocument {
            context,
            id: did.clone(),
            verification_method,
            authentication: Some(get_relationship(KeyUsage::AuthenticationKey)),
            assertion_method: Some(get_relationship(KeyUsage::IssuingKey)),
            key_agreement: Some(get_relationship(KeyUsage::KeyAgreementKey)),
            capability_invocation: Some(get_relationship(KeyUsage::CapabilityInvocationKey)),
            capability_delegation: Some(get_relationship(KeyUsage::CapabilityDelegationKey)),
            service: Some(self.services.iter().map(transform_service).collect()),
        }
    }
}

fn transform_key_jwk(did: &Did, key: &operation::PublicKey) -> Option<VerificationMethod> {
    match &key.data {
        operation::PublicKeyData::Master { .. } => None,
        operation::PublicKeyData::Vdr { .. } => None,
        operation::PublicKeyData::Other { data, .. } => {
            let jwk = data.encode_jwk();
            Some(VerificationMethod {
                id: format!("{}#{}", did, key.id),
                r#type: "JsonWebKey2020".to_string(),
                controller: did.to_string(),
                public_key_jwk: Some(jwk),
            })
        }
    }
}

fn transform_service(service: &operation::Service) -> Service {
    let r#type = match &service.r#type {
        operation::ServiceType::Value(name) => ServiceType::Str(name.to_string()),
        operation::ServiceType::List(names) => ServiceType::List(names.iter().map(|i| i.to_string()).collect()),
    };
    let transform_endpoint_value = |uri: &operation::ServiceEndpointValue| -> StringOrMap {
        match &uri {
            operation::ServiceEndpointValue::Uri(uri) => StringOrMap::Str(uri.to_string()),
            operation::ServiceEndpointValue::Json(obj) => StringOrMap::Map(obj.clone()),
        }
    };
    let service_endpoint = match &service.service_endpoint {
        operation::ServiceEndpoint::Value(endpoint) => ServiceEndpoint::StrOrMap(transform_endpoint_value(endpoint)),
        operation::ServiceEndpoint::List(endpoints) => {
            ServiceEndpoint::List(endpoints.iter().map(transform_endpoint_value).collect())
        }
    };
    Service {
        id: service.id.to_string(),
        r#type,
        service_endpoint,
    }
}
