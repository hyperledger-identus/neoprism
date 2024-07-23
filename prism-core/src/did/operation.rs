use std::str::FromStr;
use std::sync::OnceLock;

use regex::Regex;

use super::{CanonicalPrismDid, DidParsingError};
use crate::crypto::ed25519::Ed25519PublicKey;
use crate::crypto::secp256k1::Secp256k1PublicKey;
use crate::crypto::x25519::X25519PublicKey;
use crate::crypto::{ToPublicKey, ToPublicKeyError};
use crate::prelude::{AtalaOperation, SignedAtalaOperation};
use crate::proto::atala_operation::Operation;
use crate::proto::public_key::KeyData;
use crate::proto::update_did_action::Action;
use crate::proto::{self, CreateDidOperation, DeactivateDidOperation, UpdateDidAction, UpdateDidOperation};
use crate::protocol::ProtocolParameter;
use crate::utils::hash::Sha256Digest;
use crate::utils::{is_slice_unique, is_uri, is_uri_fragment};

#[derive(Debug, thiserror::Error)]
pub enum GetDidFromOperation {
    #[error("Unable to parse Did from operation: {0}")]
    DidParseError(#[from] DidParsingError),
    #[error("Operation is empty")]
    EmptyOperation,
}

pub fn get_did_from_operation(atala_operation: &AtalaOperation) -> Result<CanonicalPrismDid, GetDidFromOperation> {
    match &atala_operation.operation {
        Some(Operation::CreateDid(_)) => Ok(CanonicalPrismDid::from_operation(atala_operation)?),
        Some(Operation::UpdateDid(op)) => Ok(CanonicalPrismDid::from_suffix_str(&op.id)?),
        Some(Operation::DeactivateDid(op)) => Ok(CanonicalPrismDid::from_suffix_str(&op.id)?),
        Some(Operation::ProtocolVersionUpdate(op)) => Ok(CanonicalPrismDid::from_suffix_str(&op.proposer_did)?),
        None => Err(GetDidFromOperation::EmptyOperation),
    }
}

pub fn get_did_from_signed_operation(
    signed_operation: &SignedAtalaOperation,
) -> Result<CanonicalPrismDid, GetDidFromOperation> {
    match &signed_operation.operation {
        Some(operation) => get_did_from_operation(operation),
        None => Err(GetDidFromOperation::EmptyOperation),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CreateOperationParsingError {
    #[error("Missing did_data in create operation")]
    MissingDidData,
    #[error(transparent)]
    InvalidPublicKey(#[from] PublicKeyParsingError),
    #[error(transparent)]
    InvalidService(#[from] ServiceParsingError),
    #[error("Too many public keys")]
    TooManyPublicKeys,
    #[error("No master key found")]
    NoMasterKey,
    #[error("Too many services")]
    TooManyServices,
    #[error("Duplicate context")]
    DuplicateContext,
}

#[derive(Debug, Clone)]
pub struct CreateOperation {
    pub public_keys: Vec<PublicKey>,
    pub services: Vec<Service>,
    pub context: Vec<String>,
}

impl CreateOperation {
    pub fn parse(
        param: &ProtocolParameter,
        operation: &CreateDidOperation,
    ) -> Result<Self, CreateOperationParsingError> {
        let Some(did_data) = &operation.did_data else {
            Err(CreateOperationParsingError::MissingDidData)?
        };

        let public_keys = did_data
            .public_keys
            .iter()
            .map(|pk| PublicKey::parse(pk, param))
            .collect::<Result<Vec<_>, _>>()?;
        let services = did_data
            .services
            .iter()
            .map(|s| Service::parse(s, param))
            .collect::<Result<Vec<_>, _>>()?;
        let context = did_data.context.clone();

        Self::validate_public_key_list(param, &public_keys)?;
        Self::validate_service_list(param, &services)?;
        Self::validate_context_list(&context)?;

        Ok(Self {
            public_keys,
            services,
            context,
        })
    }

    fn validate_public_key_list(
        param: &ProtocolParameter,
        public_keys: &[PublicKey],
    ) -> Result<(), CreateOperationParsingError> {
        if public_keys.len() > param.max_public_keys {
            Err(CreateOperationParsingError::TooManyPublicKeys)?
        }

        if !public_keys.iter().any(|i| i.usage() == KeyUsage::MasterKey) {
            Err(CreateOperationParsingError::NoMasterKey)?
        }

        Ok(())
    }

    fn validate_service_list(
        param: &ProtocolParameter,
        services: &[Service],
    ) -> Result<(), CreateOperationParsingError> {
        if services.len() > param.max_services {
            return Err(CreateOperationParsingError::TooManyServices);
        }

        Ok(())
    }

    fn validate_context_list(contexts: &[String]) -> Result<(), CreateOperationParsingError> {
        if is_slice_unique(contexts) {
            Ok(())
        } else {
            Err(CreateOperationParsingError::DuplicateContext)?
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateOperationParsingError {
    #[error(transparent)]
    InvalidDidId(#[from] DidParsingError),
    #[error("Invalid previous operation hash: {0}")]
    InvalidPreviousOperationHash(String),
    #[error("Update action is malformed: {0}")]
    MalformedUpdateAction(String),
    #[error(transparent)]
    PublicKeyParsingError(#[from] PublicKeyParsingError),
    #[error(transparent)]
    ServiceParsingError(#[from] ServiceParsingError),
    #[error(transparent)]
    ServiceTypeParsingError(#[from] ServiceTypeParsingError),
    #[error(transparent)]
    ServiceEndpointParsingError(#[from] ServiceEndpointParsingError),
    #[error("Empty update action")]
    EmptyUpdateAction,
}

#[derive(Debug, Clone)]
pub struct UpdateOperation {
    pub id: CanonicalPrismDid,
    pub prev_operation_hash: Sha256Digest,
    pub actions: Vec<UpdateOperationAction>,
}

impl UpdateOperation {
    pub fn parse(
        param: &ProtocolParameter,
        operation: &UpdateDidOperation,
    ) -> Result<Self, UpdateOperationParsingError> {
        if operation.actions.is_empty() {
            Err(UpdateOperationParsingError::EmptyUpdateAction)?
        }

        let id = CanonicalPrismDid::from_suffix_str(&operation.id)?;
        let prev_operation_hash = Sha256Digest::from_bytes(&operation.previous_operation_hash)
            .map_err(UpdateOperationParsingError::InvalidPreviousOperationHash)?;

        let actions = operation
            .actions
            .iter()
            .map(|action| UpdateOperationAction::parse(action, param))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(Self {
            id,
            prev_operation_hash,
            actions,
        })
    }
}

#[derive(Debug, Clone)]
pub enum UpdateOperationAction {
    AddKey(PublicKey),
    RemoveKey(PublicKeyId),
    AddService(Service),
    RemoveService(ServiceId),
    UpdateService {
        id: ServiceId,
        r#type: Option<ServiceType>,
        service_endpoints: Option<ServiceEndpoint>,
    },
    PatchContext(Vec<String>),
}

impl UpdateOperationAction {
    pub fn parse(
        action: &UpdateDidAction,
        param: &ProtocolParameter,
    ) -> Result<Option<Self>, UpdateOperationParsingError> {
        let Some(action) = &action.action else {
            return Ok(None);
        };

        let action = match action {
            Action::AddKey(add_key) => match &add_key.key {
                Some(pk) => {
                    let parsed_key = PublicKey::parse(pk, param)?;
                    Self::AddKey(parsed_key)
                }
                None => Err(UpdateOperationParsingError::MalformedUpdateAction(
                    "AddKey action must have key property".to_owned(),
                ))?,
            },
            Action::RemoveKey(remove_key) => {
                let key_id = PublicKeyId::parse(&remove_key.key_id, param.max_id_size).map_err(|e| {
                    UpdateOperationParsingError::MalformedUpdateAction(format!(
                        "Public key id cannot be parsed ({})",
                        e
                    ))
                })?;
                Self::RemoveKey(key_id)
            }
            Action::AddService(add_service) => match &add_service.service {
                Some(service) => {
                    let parsed_service = Service::parse(service, param)?;
                    Self::AddService(parsed_service)
                }
                None => Err(UpdateOperationParsingError::MalformedUpdateAction(
                    "AddService action must have service property".to_owned(),
                ))?,
            },
            Action::RemoveService(remove_service) => {
                let service_id = ServiceId::parse(&remove_service.service_id, param.max_id_size).map_err(|_| {
                    UpdateOperationParsingError::MalformedUpdateAction("Service id cannot be parsed".to_string())
                })?;
                Self::RemoveService(service_id)
            }
            Action::UpdateService(update_service) => {
                let service_id = ServiceId::parse(&update_service.service_id, param.max_id_size).map_err(|_| {
                    UpdateOperationParsingError::MalformedUpdateAction("Service id cannot be parsed".to_string())
                })?;
                let service_type = match Some(update_service.r#type.clone()).filter(|i| !i.is_empty()) {
                    Some(s) => Some(
                        ServiceType::parse(&s, param).map_err(UpdateOperationParsingError::ServiceTypeParsingError)?,
                    ),
                    None => None,
                };
                let service_endpoints = match Some(update_service.service_endpoints.clone()).filter(|i| !i.is_empty()) {
                    Some(s) => Some(
                        ServiceEndpoint::parse(&s, param)
                            .map_err(UpdateOperationParsingError::ServiceEndpointParsingError)?,
                    ),
                    None => None,
                };
                Self::UpdateService {
                    id: service_id,
                    r#type: service_type,
                    service_endpoints,
                }
            }
            Action::PatchContext(patch_ctx) => Self::PatchContext(patch_ctx.context.clone()),
        };

        Ok(Some(action))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DeactivateOperationParsingError {
    #[error("Invalid did id: {0}")]
    InvalidDidId(#[from] DidParsingError),
    #[error("Invalid previous operation hash: {0}")]
    InvalidPreviousOperationHash(String),
}

#[derive(Debug, Clone)]
pub struct DeactivateOperation {
    pub id: CanonicalPrismDid,
    pub prev_operation_hash: Sha256Digest,
}

impl DeactivateOperation {
    pub fn parse(operation: &DeactivateDidOperation) -> Result<Self, DeactivateOperationParsingError> {
        let id = CanonicalPrismDid::from_suffix_str(&operation.id)?;
        let prev_operation_hash = Sha256Digest::from_bytes(&operation.previous_operation_hash)
            .map_err(DeactivateOperationParsingError::InvalidPreviousOperationHash)?;

        Ok(Self {
            id,
            prev_operation_hash,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub struct PublicKeyId(String);

impl PublicKeyId {
    pub fn parse(id: &str, max_length: usize) -> Result<Self, String> {
        let is_fragment = is_uri_fragment(id);
        let is_non_empty = !id.is_empty();
        let is_within_max_size = id.len() <= max_length;
        if is_fragment && is_non_empty && is_within_max_size {
            Ok(Self(id.to_owned()))
        } else {
            Err(format!(
                "Public key id must be non-empty URI fragment with max length of {}. Got {}",
                max_length, id
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub struct ServiceId(String);

impl ServiceId {
    pub fn parse(id: &str, max_length: usize) -> Result<Self, String> {
        let is_fragment = is_uri_fragment(id);
        let is_non_empty = !id.is_empty();
        let is_within_max_size = id.len() <= max_length;
        if is_fragment && is_non_empty && is_within_max_size {
            Ok(Self(id.to_owned()))
        } else {
            Err(format!(
                "Service id must be non-empty URI fragment with max length of {}. Got {}",
                max_length, id
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PublicKeyParsingError {
    #[error("Invalid key id: {0}")]
    InvalidKeyId(String),
    #[error("Missing key_data on key id {0:?}")]
    MissingKeyData(PublicKeyId),
    #[error("Unknown key usage on key id {0:?}")]
    UnknownKeyUsage(PublicKeyId),
    #[error("Unable to parse key_data on key id {id:?}: {msg}")]
    KeyDataParseError { id: PublicKeyId, msg: String },
    #[error("Master key must have type of secp256k1. (id {0:?}) ")]
    InvalidMasterKeyType(PublicKeyId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicKey {
    pub id: PublicKeyId,
    pub data: PublicKeyData,
}

impl PublicKey {
    pub fn parse(public_key: &proto::PublicKey, param: &ProtocolParameter) -> Result<Self, PublicKeyParsingError> {
        let id = PublicKeyId::parse(&public_key.id, param.max_id_size).map_err(PublicKeyParsingError::InvalidKeyId)?;
        let usage = KeyUsage::parse(&public_key.usage()).ok_or(PublicKeyParsingError::UnknownKeyUsage(id.clone()))?;
        let Some(key_data) = &public_key.key_data else {
            Err(PublicKeyParsingError::MissingKeyData(id))?
        };
        let pk = SupportedPublicKey::from_key_data(key_data).map_err(|e| PublicKeyParsingError::KeyDataParseError {
            id: id.clone(),
            msg: e.to_string(),
        })?;
        let data = match (usage, pk) {
            (KeyUsage::MasterKey, SupportedPublicKey::Secp256k1(pk)) => PublicKeyData::Master { data: pk },
            (KeyUsage::MasterKey, _) => Err(PublicKeyParsingError::InvalidMasterKeyType(id.clone()))?,
            (usage, pk) => PublicKeyData::Other { data: pk, usage },
        };

        Ok(Self { id, data })
    }

    pub fn usage(&self) -> KeyUsage {
        match &self.data {
            PublicKeyData::Master { .. } => KeyUsage::MasterKey,
            PublicKeyData::Other { usage, .. } => *usage,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SupportedPublicKeyError {
    #[error(transparent)]
    Parse {
        #[from]
        source: ToPublicKeyError,
    },
    #[error("Unsupported curve {curve}")]
    UnsupportedCurve { curve: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportedPublicKey {
    Secp256k1(Secp256k1PublicKey),
    Ed25519(Ed25519PublicKey),
    X25519(X25519PublicKey),
}

impl SupportedPublicKey {
    pub fn from_key_data(key_data: &KeyData) -> Result<Self, SupportedPublicKeyError> {
        let curve_name: &str = match key_data {
            KeyData::EcKeyData(k) => &k.curve,
            KeyData::CompressedEcKeyData(k) => &k.curve,
        };

        match curve_name {
            "secp256k1" => Ok(Self::Secp256k1(Self::convert_secp256k1(key_data)?)),
            "ed25519" => Ok(Self::Ed25519(Self::convert_ed25519(key_data)?)),
            "x25519" => Ok(Self::X25519(Self::convert_x25519(key_data)?)),
            c => Err(SupportedPublicKeyError::UnsupportedCurve { curve: c.to_string() }),
        }
    }

    fn convert_secp256k1(key_data: &KeyData) -> Result<Secp256k1PublicKey, ToPublicKeyError> {
        let pk = match key_data {
            KeyData::EcKeyData(k) => {
                let mut data = Vec::with_capacity(65);
                data.push(0x04);
                data.extend_from_slice(k.x.as_ref());
                data.extend_from_slice(k.y.as_ref());
                data.to_public_key()?
            }
            KeyData::CompressedEcKeyData(k) => k.data.to_public_key()?,
        };
        Ok(pk)
    }

    fn convert_ed25519(key_data: &KeyData) -> Result<Ed25519PublicKey, ToPublicKeyError> {
        let pk = match key_data {
            KeyData::EcKeyData(k) => k.x.to_public_key()?,
            KeyData::CompressedEcKeyData(k) => k.data.to_public_key()?,
        };
        Ok(pk)
    }

    fn convert_x25519(key_data: &KeyData) -> Result<X25519PublicKey, ToPublicKeyError> {
        let pk = match key_data {
            KeyData::EcKeyData(k) => k.x.to_public_key()?,
            KeyData::CompressedEcKeyData(k) => k.data.to_public_key()?,
        };
        Ok(pk)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicKeyData {
    Master { data: Secp256k1PublicKey },
    Other { data: SupportedPublicKey, usage: KeyUsage },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyUsage {
    MasterKey,
    IssuingKey,
    KeyAgreementKey,
    AuthenticationKey,
    RevocationKey,
    CapabilityInvocationKey,
    CapabilityDelegationKey,
}

impl KeyUsage {
    pub fn parse(usage: &proto::KeyUsage) -> Option<Self> {
        match usage {
            proto::KeyUsage::MasterKey => Some(Self::MasterKey),
            proto::KeyUsage::IssuingKey => Some(Self::IssuingKey),
            proto::KeyUsage::KeyAgreementKey => Some(Self::KeyAgreementKey),
            proto::KeyUsage::AuthenticationKey => Some(Self::AuthenticationKey),
            proto::KeyUsage::RevocationKey => Some(Self::RevocationKey),
            proto::KeyUsage::CapabilityInvocationKey => Some(Self::CapabilityInvocationKey),
            proto::KeyUsage::CapabilityDelegationKey => Some(Self::CapabilityDelegationKey),
            proto::KeyUsage::UnknownKey => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceParsingError {
    #[error("Invalid service id: {0}")]
    InvalidServiceId(String),
    #[error(transparent)]
    InvalidServiceType(#[from] ServiceTypeParsingError),
    #[error(transparent)]
    InvalidServiceEndpoint(#[from] ServiceEndpointParsingError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    pub id: ServiceId,
    pub r#type: ServiceType,
    pub service_endpoints: ServiceEndpoint,
}

impl Service {
    pub fn parse(service: &proto::Service, param: &ProtocolParameter) -> Result<Self, ServiceParsingError> {
        let id = ServiceId::parse(&service.id, param.max_id_size).map_err(ServiceParsingError::InvalidServiceId)?;
        let r#type = ServiceType::parse(&service.r#type, param).map_err(ServiceParsingError::InvalidServiceType)?;
        let service_endpoints = ServiceEndpoint::parse(&service.service_endpoint, param)
            .map_err(ServiceParsingError::InvalidServiceEndpoint)?;

        Ok(Self {
            id,
            r#type,
            service_endpoints,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceTypeParsingError {
    #[error(transparent)]
    InvalidValue(#[from] ServiceTypeNameParsingError),
    #[error("Service type string must not exceed {limit} characters. Got {actual}.")]
    TooLong { limit: usize, actual: usize },
    #[error("Service type must not be an empty array")]
    EmptyList,
    #[error("Service type does not conform to the ABNF rule")]
    NotConformToABNF,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceType {
    Value(ServiceTypeName),
    List(Vec<ServiceTypeName>),
}

impl ServiceType {
    pub fn parse(service_type: &str, param: &ProtocolParameter) -> Result<Self, ServiceTypeParsingError> {
        if service_type.len() > param.max_type_size {
            Err(ServiceTypeParsingError::TooLong {
                limit: param.max_type_size,
                actual: service_type.len(),
            })?
        }

        // try parse as json list of strings
        let parsed: Result<Vec<String>, _> = serde_json::from_str(service_type);
        if let Ok(list) = parsed {
            if list.is_empty() {
                Err(ServiceTypeParsingError::EmptyList)?
            }

            if service_type != serde_json::to_string(&list).expect("Serializing Vec<String> to JSON must not fail!") {
                Err(ServiceTypeParsingError::NotConformToABNF)?
            }

            let names: Result<Vec<ServiceTypeName>, _> = list.iter().map(|i| ServiceTypeName::from_str(i)).collect();

            return Ok(Self::List(names?));
        }

        // try parse as single string
        let name = ServiceTypeName::from_str(service_type)?;
        Ok(Self::Value(name))
    }
}

static SERVICE_TYPE_NAME_RE: OnceLock<Regex> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
#[error("The string {0} is not a valid serviceType name")]
pub struct ServiceTypeNameParsingError(String);

#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub struct ServiceTypeName(String);

impl FromStr for ServiceTypeName {
    type Err = ServiceTypeNameParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let regex = SERVICE_TYPE_NAME_RE.get_or_init(|| {
            Regex::new(r"^[A-Za-z0-9\-_]+(\s*[A-Za-z0-9\-_])*$").expect("ServiceTypeName regex is invalid")
        });
        if regex.is_match(s) {
            Ok(Self(s.to_owned()))
        } else {
            Err(ServiceTypeNameParsingError(s.to_owned()))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceEndpointParsingError {
    #[error(transparent)]
    InvalidValue(#[from] ServiceEndpointValueParsingError),
    #[error("Service endpoint string must not exceed {limit} characters. Got {actual}.")]
    TooLong { limit: usize, actual: usize },
    #[error("Service endpoint must not be an empty array")]
    EmptyList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceEndpoint {
    Value(ServiceEndpointValue),
    List(Vec<ServiceEndpointValue>),
}

impl ServiceEndpoint {
    pub fn parse(service_endpoint: &str, param: &ProtocolParameter) -> Result<Self, ServiceEndpointParsingError> {
        if service_endpoint.len() > param.max_service_endpoint_size {
            Err(ServiceEndpointParsingError::TooLong {
                limit: param.max_service_endpoint_size,
                actual: service_endpoint.len(),
            })?
        }

        // try parse as json object
        let parsed_map = serde_json::from_str(service_endpoint);
        if let Ok(json) = parsed_map {
            return Ok(Self::Value(ServiceEndpointValue::Json(json)));
        }

        // try parse as json array
        let parsed_array = serde_json::from_str::<Vec<serde_json::Value>>(service_endpoint);
        if let Ok(list) = parsed_array {
            if list.is_empty() {
                Err(ServiceEndpointParsingError::EmptyList)?
            }

            let endpoints: Result<Vec<ServiceEndpointValue>, _> =
                list.into_iter().map(ServiceEndpointValue::try_from).collect();
            return Ok(Self::List(endpoints?));
        }

        // try parse as single string
        Ok(Self::Value(ServiceEndpointValue::from_str(service_endpoint)?))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceEndpointValueParsingError {
    #[error("Fail to parse '{uri}' as a URI")]
    InvalidUri { uri: String },
    #[error("ServiceEndpoint is not a URI string nor JSON object. Got {0}.")]
    InvalidJsonType(serde_json::Value),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceEndpointValue {
    Uri(String),
    Json(serde_json::Map<String, serde_json::Value>),
}

impl FromStr for ServiceEndpointValue {
    type Err = ServiceEndpointValueParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if is_uri(s) {
            Ok(Self::Uri(s.to_owned()))
        } else {
            Err(ServiceEndpointValueParsingError::InvalidUri { uri: s.to_string() })
        }
    }
}

impl TryFrom<serde_json::Value> for ServiceEndpointValue {
    type Error = ServiceEndpointValueParsingError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::String(s) => Self::from_str(&s),
            serde_json::Value::Object(map) => Ok(Self::Json(map)),
            _ => Err(ServiceEndpointValueParsingError::InvalidJsonType(value)),
        }
    }
}
