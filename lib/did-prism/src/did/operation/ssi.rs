use std::str::FromStr;
use std::sync::LazyLock;

use enum_dispatch::enum_dispatch;
use identus_apollo::crypto::Error as CryptoError;
use identus_apollo::crypto::ed25519::Ed25519PublicKey;
use identus_apollo::crypto::secp256k1::Secp256k1PublicKey;
use identus_apollo::crypto::x25519::X25519PublicKey;
use identus_apollo::hash::Sha256Digest;
use identus_apollo::jwk::EncodeJwk;
use identus_did_core::utils::uri::{is_uri, is_uri_fragment};
use regex::Regex;

use crate::did::CanonicalPrismDid;
use crate::did::error::{
    CreateDidOperationError, DeactivateDidOperationError, PublicKeyError, PublicKeyIdError, ServiceEndpointError,
    ServiceError, ServiceIdError, ServiceTypeError, UpdateDidOperationError,
};
use crate::did::operation::OperationParameters;
use crate::error::InvalidInputSizeError;
use crate::proto::prism_ssi::public_key::Key_data;
use crate::proto::prism_ssi::update_didaction::Action;
use crate::proto::prism_ssi::{ProtoCreateDID, ProtoDeactivateDID, ProtoUpdateDID, UpdateDIDAction};
use crate::utils::is_slice_unique;
use crate::{location, proto};

static SERVICE_TYPE_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9\-_]+(\s*[A-Za-z0-9\-_])*$").expect("ServiceTypeName regex is invalid"));

#[derive(Debug, Clone)]
pub struct CreateDidOperation {
    pub public_keys: Vec<PublicKey>,
    pub services: Vec<Service>,
    pub context: Vec<String>,
}

impl CreateDidOperation {
    pub fn parse(param: &OperationParameters, operation: &ProtoCreateDID) -> Result<Self, CreateDidOperationError> {
        let Some(did_data) = operation.did_data.as_ref() else {
            Err(CreateDidOperationError::MissingDidData)?
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
        param: &OperationParameters,
        public_keys: &[PublicKey],
    ) -> Result<(), CreateDidOperationError> {
        if public_keys.len() > param.max_public_keys {
            Err(CreateDidOperationError::TooManyPublicKeys {
                source: InvalidInputSizeError::TooBig {
                    limit: param.max_public_keys,
                    actual: public_keys.len(),
                    type_name: std::any::type_name_of_val(public_keys),
                    location: location!(),
                },
            })?
        }

        if !public_keys.iter().any(|i| i.data.usage() == KeyUsage::MasterKey) {
            Err(CreateDidOperationError::MissingMasterKey)?
        }

        Ok(())
    }

    fn validate_service_list(param: &OperationParameters, services: &[Service]) -> Result<(), CreateDidOperationError> {
        if services.len() > param.max_services {
            return Err(CreateDidOperationError::TooManyServices {
                source: InvalidInputSizeError::TooBig {
                    limit: param.max_public_keys,
                    actual: services.len(),
                    type_name: std::any::type_name_of_val(services),
                    location: location!(),
                },
            });
        }

        Ok(())
    }

    fn validate_context_list(contexts: &[String]) -> Result<(), CreateDidOperationError> {
        if is_slice_unique(contexts) {
            Ok(())
        } else {
            Err(CreateDidOperationError::DuplicateContext)?
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateDidOperation {
    pub id: CanonicalPrismDid,
    pub prev_operation_hash: Sha256Digest,
    pub actions: Vec<UpdateOperationAction>,
}

impl UpdateDidOperation {
    pub fn parse(param: &OperationParameters, operation: &ProtoUpdateDID) -> Result<Self, UpdateDidOperationError> {
        if operation.actions.is_empty() {
            Err(UpdateDidOperationError::EmptyAction)?
        }

        let id = CanonicalPrismDid::from_suffix_str(&operation.id)?;
        let prev_operation_hash = Sha256Digest::from_bytes(&operation.previous_operation_hash)
            .map_err(|e| UpdateDidOperationError::InvalidPreviousOperationHash { source: e })?;

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
        action: &UpdateDIDAction,
        param: &OperationParameters,
    ) -> Result<Option<Self>, UpdateDidOperationError> {
        let Some(action) = &action.action else {
            return Ok(None);
        };

        let action = match action {
            Action::AddKey(add_key) => match add_key.key.as_ref() {
                Some(pk) => {
                    let parsed_key = PublicKey::parse(pk, param)?;
                    Self::AddKey(parsed_key)
                }
                None => Err(UpdateDidOperationError::MissingUpdateActionData {
                    action_type: std::any::type_name_of_val(add_key),
                    field_name: "key",
                })?,
            },
            Action::RemoveKey(remove_key) => {
                let key_id = PublicKeyId::parse(&remove_key.keyId, param.max_id_size).map_err(|e| {
                    UpdateDidOperationError::InvalidPublicKey {
                        source: PublicKeyError::InvalidKeyId {
                            source: e,
                            id: remove_key.keyId.clone(),
                        },
                    }
                })?;
                Self::RemoveKey(key_id)
            }
            Action::AddService(add_service) => match add_service.service.as_ref() {
                Some(service) => {
                    let parsed_service = Service::parse(service, param)?;
                    Self::AddService(parsed_service)
                }
                None => Err(UpdateDidOperationError::MissingUpdateActionData {
                    action_type: std::any::type_name_of_val(add_service),
                    field_name: "service",
                })?,
            },
            Action::RemoveService(remove_service) => {
                let service_id = ServiceId::parse(&remove_service.serviceId, param.max_id_size).map_err(|e| {
                    UpdateDidOperationError::InvalidService {
                        source: ServiceError::InvalidServiceId {
                            source: e,
                            id: remove_service.serviceId.clone(),
                        },
                    }
                })?;
                Self::RemoveService(service_id)
            }
            Action::UpdateService(update_service) => {
                let service_id = ServiceId::parse(&update_service.serviceId, param.max_id_size).map_err(|e| {
                    UpdateDidOperationError::InvalidService {
                        source: ServiceError::InvalidServiceId {
                            source: e,
                            id: update_service.serviceId.clone(),
                        },
                    }
                })?;
                let service_type =
                    match Some(update_service.type_.clone()).filter(|i| !i.is_empty()) {
                        Some(s) => Some(ServiceType::parse(&s, param).map_err(|e| {
                            UpdateDidOperationError::InvalidService {
                                source: ServiceError::InvalidServiceType {
                                    source: e,
                                    type_name: update_service.type_.clone(),
                                },
                            }
                        })?),
                        None => None,
                    };
                let service_endpoints = match Some(update_service.service_endpoints.clone()).filter(|i| !i.is_empty()) {
                    Some(s) => Some(ServiceEndpoint::parse(&s, param).map_err(|e| {
                        UpdateDidOperationError::InvalidService {
                            source: ServiceError::InvalidServiceEndpoint {
                                source: e,
                                endpoint: update_service.service_endpoints.clone(),
                            },
                        }
                    })?),
                    None => None,
                };
                Self::UpdateService {
                    id: service_id,
                    r#type: service_type,
                    service_endpoints,
                }
            }
            Action::PatchContext(patch_ctx) => {
                let context = patch_ctx.context.clone();
                Self::validate_context_list(&context)?;
                Self::PatchContext(context)
            }
        };

        Ok(Some(action))
    }

    fn validate_context_list(contexts: &[String]) -> Result<(), UpdateDidOperationError> {
        if is_slice_unique(contexts) {
            Ok(())
        } else {
            Err(UpdateDidOperationError::DuplicateContext)?
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeactivateDidOperation {
    pub id: CanonicalPrismDid,
    pub prev_operation_hash: Sha256Digest,
}

impl DeactivateDidOperation {
    pub fn parse(operation: &ProtoDeactivateDID) -> Result<Self, DeactivateDidOperationError> {
        let id = CanonicalPrismDid::from_suffix_str(&operation.id)?;
        let prev_operation_hash = Sha256Digest::from_bytes(&operation.previous_operation_hash)
            .map_err(|e| DeactivateDidOperationError::InvalidPreviousOperationHash { source: e })?;

        Ok(Self {
            id,
            prev_operation_hash,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub struct PublicKeyId(String);

impl PublicKeyId {
    pub fn parse(id: &str, max_length: usize) -> Result<Self, PublicKeyIdError> {
        if id.is_empty() {
            return Err(PublicKeyIdError::Empty);
        }

        if id.len() > max_length {
            return Err(PublicKeyIdError::TooLong {
                source: InvalidInputSizeError::TooBig {
                    limit: max_length,
                    actual: id.len(),
                    type_name: std::any::type_name::<Self>(),
                    location: location!(),
                },
            });
        }

        if !is_uri_fragment(id) {
            return Err(PublicKeyIdError::InvalidUriFragment);
        }

        Ok(Self(id.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub struct ServiceId(String);

impl ServiceId {
    pub fn parse(id: &str, max_length: usize) -> Result<Self, ServiceIdError> {
        if id.is_empty() {
            return Err(ServiceIdError::Empty);
        }

        if id.len() > max_length {
            return Err(ServiceIdError::TooLong {
                source: InvalidInputSizeError::TooBig {
                    limit: max_length,
                    actual: id.len(),
                    type_name: std::any::type_name::<Self>(),
                    location: location!(),
                },
            });
        }

        if !is_uri_fragment(id) {
            return Err(ServiceIdError::InvalidUriFragment);
        }

        Ok(Self(id.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PublicKey {
    pub id: PublicKeyId,
    pub data: PublicKeyData,
    pub orig: proto::prism_ssi::PublicKey,
}

impl PublicKey {
    pub fn parse(
        public_key: &proto::prism_ssi::PublicKey,
        param: &OperationParameters,
    ) -> Result<Self, PublicKeyError> {
        let id = PublicKeyId::parse(&public_key.id, param.max_id_size).map_err(|e| PublicKeyError::InvalidKeyId {
            source: e,
            id: public_key.id.to_string(),
        })?;
        let usage = KeyUsage::parse(
            &public_key
                .usage
                .enum_value()
                .map_err(|_| PublicKeyError::UnknownKeyUsage { id: id.clone() })?,
        )
        .ok_or(PublicKeyError::UnknownKeyUsage { id: id.clone() })?;
        let Some(key_data) = &public_key.key_data else {
            Err(PublicKeyError::MissingKeyData { id: id.clone() })?
        };
        let pk = NonOperationPublicKey::parse(key_data)
            .map_err(|e| PublicKeyError::InvalidKeyData {
                source: e,
                id: id.clone(),
            })?
            .ok_or_else(|| PublicKeyError::UnsupportedCurve { id: id.clone() })?;
        let data = match (usage, pk) {
            (KeyUsage::MasterKey, NonOperationPublicKey::Secp256k1(pk)) => PublicKeyData::Master { data: pk },
            (KeyUsage::MasterKey, _) => Err(PublicKeyError::MasterKeyNotSecp256k1 { id: id.clone() })?,
            (KeyUsage::VdrKey, NonOperationPublicKey::Secp256k1(pk)) => PublicKeyData::Vdr { data: pk },
            (KeyUsage::VdrKey, _) => Err(PublicKeyError::VdrKeyNotSecp256k1 { id: id.clone() })?,
            (usage, pk) => PublicKeyData::Other { data: pk, usage },
        };

        Ok(Self {
            id,
            data,
            orig: public_key.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[enum_dispatch(EncodeVec)]
pub enum NonOperationPublicKey {
    Secp256k1(Secp256k1PublicKey),
    Ed25519(Ed25519PublicKey),
    X25519(X25519PublicKey),
}

impl NonOperationPublicKey {
    pub fn parse(key_data: &Key_data) -> Result<Option<Self>, CryptoError> {
        let curve_name: &str = match key_data {
            Key_data::EcKeyData(k) => &k.curve,
            Key_data::CompressedEcKeyData(k) => &k.curve,
        };

        match curve_name {
            "secp256k1" => Ok(Some(Self::Secp256k1(Self::convert_secp256k1(key_data)?))),
            "Ed25519" => Ok(Some(Self::Ed25519(Self::convert_ed25519(key_data)?))),
            "X25519" => Ok(Some(Self::X25519(Self::convert_x25519(key_data)?))),
            _ => Ok(None),
        }
    }

    fn convert_secp256k1(key_data: &Key_data) -> Result<Secp256k1PublicKey, CryptoError> {
        let pk = match key_data {
            Key_data::EcKeyData(k) => {
                let mut data = Vec::with_capacity(65);
                data.push(0x04);
                data.extend_from_slice(k.x.as_ref());
                data.extend_from_slice(k.y.as_ref());
                Secp256k1PublicKey::from_slice(&data)?
            }
            Key_data::CompressedEcKeyData(k) => Secp256k1PublicKey::from_slice(&k.data)?,
        };
        Ok(pk)
    }

    fn convert_ed25519(key_data: &Key_data) -> Result<Ed25519PublicKey, CryptoError> {
        let pk = match key_data {
            Key_data::EcKeyData(k) => Ed25519PublicKey::from_slice(&k.x)?,
            Key_data::CompressedEcKeyData(k) => Ed25519PublicKey::from_slice(&k.data)?,
        };
        Ok(pk)
    }

    fn convert_x25519(key_data: &Key_data) -> Result<X25519PublicKey, CryptoError> {
        let pk = match key_data {
            Key_data::EcKeyData(k) => X25519PublicKey::from_slice(&k.x)?,
            Key_data::CompressedEcKeyData(k) => X25519PublicKey::from_slice(&k.data)?,
        };
        Ok(pk)
    }
}

impl EncodeJwk for NonOperationPublicKey {
    fn encode_jwk(&self) -> identus_apollo::jwk::Jwk {
        match self {
            NonOperationPublicKey::Secp256k1(pk) => pk.encode_jwk(),
            NonOperationPublicKey::Ed25519(pk) => pk.encode_jwk(),
            NonOperationPublicKey::X25519(pk) => pk.encode_jwk(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublicKeyData {
    Master {
        data: Secp256k1PublicKey,
    },
    Vdr {
        data: Secp256k1PublicKey,
    },
    Other {
        data: NonOperationPublicKey,
        usage: KeyUsage,
    },
}

impl PublicKeyData {
    pub fn usage(&self) -> KeyUsage {
        match &self {
            Self::Master { .. } => KeyUsage::MasterKey,
            Self::Vdr { .. } => KeyUsage::VdrKey,
            Self::Other { usage, .. } => *usage,
        }
    }
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
    VdrKey,
}

impl KeyUsage {
    pub fn parse(usage: &proto::prism_ssi::KeyUsage) -> Option<Self> {
        match usage {
            proto::prism_ssi::KeyUsage::MASTER_KEY => Some(Self::MasterKey),
            proto::prism_ssi::KeyUsage::ISSUING_KEY => Some(Self::IssuingKey),
            proto::prism_ssi::KeyUsage::KEY_AGREEMENT_KEY => Some(Self::KeyAgreementKey),
            proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY => Some(Self::AuthenticationKey),
            proto::prism_ssi::KeyUsage::REVOCATION_KEY => Some(Self::RevocationKey),
            proto::prism_ssi::KeyUsage::CAPABILITY_INVOCATION_KEY => Some(Self::CapabilityInvocationKey),
            proto::prism_ssi::KeyUsage::CAPABILITY_DELEGATION_KEY => Some(Self::CapabilityDelegationKey),
            proto::prism_ssi::KeyUsage::VDR_KEY => Some(Self::VdrKey),
            proto::prism_ssi::KeyUsage::UNKNOWN_KEY => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Service {
    pub id: ServiceId,
    pub r#type: ServiceType,
    pub service_endpoint: ServiceEndpoint,
    pub orig: proto::prism_ssi::Service,
}

impl Service {
    pub fn parse(service: &proto::prism_ssi::Service, param: &OperationParameters) -> Result<Self, ServiceError> {
        let id = ServiceId::parse(&service.id, param.max_id_size).map_err(|e| ServiceError::InvalidServiceId {
            source: e,
            id: service.id.to_string(),
        })?;
        let r#type = ServiceType::parse(&service.type_, param).map_err(|e| ServiceError::InvalidServiceType {
            source: e,
            type_name: service.type_.to_string(),
        })?;
        let service_endpoint = ServiceEndpoint::parse(&service.service_endpoint, param).map_err(|e| {
            ServiceError::InvalidServiceEndpoint {
                source: e,
                endpoint: service.service_endpoint.to_string(),
            }
        })?;

        Ok(Self {
            id,
            r#type,
            service_endpoint,
            orig: service.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceType {
    Value(ServiceTypeName),
    List(Vec<ServiceTypeName>),
}

impl ServiceType {
    pub fn parse(service_type: &str, param: &OperationParameters) -> Result<Self, ServiceTypeError> {
        if service_type.len() > param.max_type_size {
            Err(ServiceTypeError::ExceedMaxSize {
                limit: param.max_type_size,
            })?
        }

        // try parse as json list of strings
        let parsed: Result<Vec<String>, _> = serde_json::from_str(service_type);
        if let Ok(list) = parsed {
            if list.is_empty() {
                Err(ServiceTypeError::Empty)?
            }

            if service_type != serde_json::to_string(&list).expect("serializing Vec<String> to JSON must not fail!") {
                Err(ServiceTypeError::InvalidSyntax)?
            }

            let names: Result<Vec<ServiceTypeName>, _> = list.iter().map(|i| ServiceTypeName::from_str(i)).collect();

            return Ok(Self::List(names?));
        }

        // try parse as single string
        let name = ServiceTypeName::from_str(service_type)?;
        Ok(Self::Value(name))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub struct ServiceTypeName(String);

impl FromStr for ServiceTypeName {
    type Err = ServiceTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if SERVICE_TYPE_NAME_RE.is_match(s) {
            Ok(Self(s.to_owned()))
        } else {
            Err(Self::Err::InvalidSyntax)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceEndpoint {
    Value(ServiceEndpointValue),
    List(Vec<ServiceEndpointValue>),
}

impl ServiceEndpoint {
    pub fn parse(service_endpoint: &str, param: &OperationParameters) -> Result<Self, ServiceEndpointError> {
        if service_endpoint.len() > param.max_service_endpoint_size {
            Err(ServiceEndpointError::ExceedMaxSize {
                limit: param.max_service_endpoint_size,
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
                Err(ServiceEndpointError::Empty)?
            }

            let endpoints: Result<Vec<ServiceEndpointValue>, _> =
                list.into_iter().map(ServiceEndpointValue::try_from).collect();
            return Ok(Self::List(endpoints?));
        }

        // try parse as single string
        Ok(Self::Value(ServiceEndpointValue::from_str(service_endpoint)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceEndpointValue {
    Uri(String),
    Json(serde_json::Map<String, serde_json::Value>),
}

impl FromStr for ServiceEndpointValue {
    type Err = ServiceEndpointError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if is_uri(s) {
            Ok(Self::Uri(s.to_owned()))
        } else {
            Err(ServiceEndpointError::InvalidSyntax)
        }
    }
}

impl TryFrom<serde_json::Value> for ServiceEndpointValue {
    type Error = ServiceEndpointError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::String(s) => Self::from_str(&s),
            serde_json::Value::Object(map) => Ok(Self::Json(map)),
            _ => Err(ServiceEndpointError::InvalidSyntax),
        }
    }
}
