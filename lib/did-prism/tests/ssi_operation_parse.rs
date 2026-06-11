//! Comprehensive tests for the SSI operation parsing module (ssi.rs).
//!
//! These tests target the parse/validation paths that are not covered by the
//! higher-level integration tests in ssi_operation.rs.

use std::str::FromStr;

use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
use identus_apollo::jwk::EncodeJwk;
use identus_did_prism::did::error::{
    CreateDidOperationError, DeactivateDidOperationError, PublicKeyError, PublicKeyIdError, ServiceEndpointError,
    ServiceError, ServiceIdError, ServiceTypeError, UpdateDidOperationError,
};
use identus_did_prism::did::operation::{
    CreateDidOperation, DeactivateDidOperation, KeyUsage, NonOperationPublicKey, OperationParameters, PublicKey,
    PublicKeyData, PublicKeyId, Service, ServiceEndpoint, ServiceEndpointValue, ServiceId, ServiceType,
    ServiceTypeValue, UpdateDidOperation, UpdateOperationAction,
};
use identus_did_prism::proto;

/// Helper: default v1 operation parameters.
fn v1_params() -> OperationParameters {
    OperationParameters::v1()
}

/// Helper: create a valid master public key proto.
fn master_key_proto(id: &str) -> proto::prism_ssi::PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let pk = sk.to_public_key();
    proto::prism_ssi::PublicKey {
        id: id.to_string(),
        usage: proto::prism_ssi::KeyUsage::MASTER_KEY.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
            proto::prism_ssi::CompressedECKeyData {
                curve: "secp256k1".to_string(),
                data: pk.encode_compressed().into(),
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    }
}

/// Helper: create a valid secp256k1 public key proto with arbitrary usage.
fn secp_key_proto(id: &str, usage: proto::prism_ssi::KeyUsage, seed: u8) -> proto::prism_ssi::PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[seed; 32]).unwrap();
    let pk = sk.to_public_key();
    proto::prism_ssi::PublicKey {
        id: id.to_string(),
        usage: usage.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
            proto::prism_ssi::CompressedECKeyData {
                curve: "secp256k1".to_string(),
                data: pk.encode_compressed().into(),
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    }
}

/// Helper: create an ed25519 public key proto.
fn ed25519_key_proto(id: &str, usage: proto::prism_ssi::KeyUsage) -> proto::prism_ssi::PublicKey {
    let pk_bytes = [42u8; 32]; // arbitrary 32-byte value for Ed25519 public key
    proto::prism_ssi::PublicKey {
        id: id.to_string(),
        usage: usage.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
            proto::prism_ssi::CompressedECKeyData {
                curve: "Ed25519".to_string(),
                data: pk_bytes.into(),
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    }
}

/// Helper: create an x25519 public key proto.
fn x25519_key_proto(id: &str, usage: proto::prism_ssi::KeyUsage) -> proto::prism_ssi::PublicKey {
    let pk_bytes = [99u8; 32]; // arbitrary 32-byte value for X25519 public key
    proto::prism_ssi::PublicKey {
        id: id.to_string(),
        usage: usage.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
            proto::prism_ssi::CompressedECKeyData {
                curve: "X25519".to_string(),
                data: pk_bytes.into(),
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    }
}

/// Helper: create an EC key data proto (uncompressed form) for secp256k1.
fn secp_key_uncompressed_proto(id: &str, usage: proto::prism_ssi::KeyUsage) -> proto::prism_ssi::PublicKey {
    let sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let pk = sk.to_public_key();
    let uncompressed = pk.encode_uncompressed();
    // uncompressed[0] is 0x04, [1..33] is x, [33..65] is y
    let x = &uncompressed[1..33];
    let y = &uncompressed[33..65];
    proto::prism_ssi::PublicKey {
        id: id.to_string(),
        usage: usage.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::EcKeyData(
            proto::prism_ssi::ECKeyData {
                curve: "secp256k1".to_string(),
                x: x.into(),
                y: y.into(),
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    }
}

/// Helper: create a valid service proto.
fn service_proto(id: &str, type_: &str, endpoint: &str) -> proto::prism_ssi::Service {
    proto::prism_ssi::Service {
        id: id.to_string(),
        type_: type_.to_string(),
        service_endpoint: endpoint.to_string(),
        special_fields: Default::default(),
    }
}

/// Helper: a valid did suffix (64 hex chars = 32 bytes).
fn valid_did_suffix() -> String {
    "a".repeat(64)
}

/// Helper: a valid previous operation hash (32 bytes).
fn valid_prev_op_hash() -> Vec<u8> {
    vec![0u8; 32]
}

// ============================================================================
// CreateDidOperation::parse tests
// ============================================================================

#[test]
fn create_did_parse_success_with_master_key_only() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoCreateDID {
        did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
            public_keys: vec![master_key_proto("master-0")],
            services: vec![],
            context: vec![],
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let result = CreateDidOperation::parse(&param, &proto).unwrap();
    assert_eq!(result.public_keys.len(), 1);
    assert_eq!(result.services.len(), 0);
    assert!(result.context.is_empty());
}

#[test]
fn create_did_parse_missing_did_data() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoCreateDID {
        did_data: None.into(),
        special_fields: Default::default(),
    };

    let err = CreateDidOperation::parse(&param, &proto).unwrap_err();
    let CreateDidOperationError::MissingDidData = err else {
        panic!("expected MissingDidData error, got: {err:?}");
    };
}

#[test]
fn create_did_parse_missing_master_key() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoCreateDID {
        did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
            public_keys: vec![secp_key_proto(
                "auth-0",
                proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY,
                2,
            )],
            services: vec![],
            context: vec![],
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let err = CreateDidOperation::parse(&param, &proto).unwrap_err();
    let CreateDidOperationError::MissingMasterKey = err else {
        panic!("expected MissingMasterKey error, got: {err:?}");
    };
}

#[test]
fn create_did_parse_too_many_public_keys() {
    let mut param = v1_params();
    param.max_public_keys = 1;

    let proto = proto::prism_ssi::ProtoCreateDID {
        did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
            public_keys: vec![
                master_key_proto("master-0"),
                secp_key_proto("auth-0", proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY, 2),
            ],
            services: vec![],
            context: vec![],
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let err = CreateDidOperation::parse(&param, &proto).unwrap_err();
    let CreateDidOperationError::TooManyPublicKeys { .. } = err else {
        panic!("expected TooManyPublicKeys error, got: {err:?}");
    };
}

#[test]
fn create_did_parse_too_many_services() {
    let mut param = v1_params();
    param.max_services = 0;

    let proto = proto::prism_ssi::ProtoCreateDID {
        did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
            public_keys: vec![master_key_proto("master-0")],
            services: vec![service_proto("svc-0", "LinkedDomains", "https://example.com")],
            context: vec![],
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let err = CreateDidOperation::parse(&param, &proto).unwrap_err();
    let CreateDidOperationError::TooManyServices { .. } = err else {
        panic!("expected TooManyServices error, got: {err:?}");
    };
}

#[test]
fn create_did_parse_duplicate_context() {
    let param = v1_params();
    let ctx = "https://www.w3.org/ns/did/v1".to_string();
    let proto = proto::prism_ssi::ProtoCreateDID {
        did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
            public_keys: vec![master_key_proto("master-0")],
            services: vec![],
            context: vec![ctx.clone(), ctx],
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let err = CreateDidOperation::parse(&param, &proto).unwrap_err();
    let CreateDidOperationError::DuplicateContext = err else {
        panic!("expected DuplicateContext error, got: {err:?}");
    };
}

#[test]
fn create_did_parse_success_with_services_and_context() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoCreateDID {
        did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
            public_keys: vec![master_key_proto("master-0")],
            services: vec![service_proto("svc-0", "LinkedDomains", "https://example.com")],
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let result = CreateDidOperation::parse(&param, &proto).unwrap();
    assert_eq!(result.services.len(), 1);
    assert_eq!(result.context.len(), 1);
}

#[test]
fn create_did_parse_with_multiple_key_types() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoCreateDID {
        did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
            public_keys: vec![
                master_key_proto("master-0"),
                secp_key_proto("vdr-0", proto::prism_ssi::KeyUsage::VDR_KEY, 2),
                secp_key_proto("auth-0", proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY, 3),
                secp_key_proto("issue-0", proto::prism_ssi::KeyUsage::ISSUING_KEY, 4),
                secp_key_proto("key-agree-0", proto::prism_ssi::KeyUsage::KEY_AGREEMENT_KEY, 5),
            ],
            services: vec![],
            context: vec![],
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    };

    let result = CreateDidOperation::parse(&param, &proto).unwrap();
    assert_eq!(result.public_keys.len(), 5);
}

// ============================================================================
// UpdateDidOperation::parse tests
// ============================================================================

#[test]
fn update_did_parse_empty_actions() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::EmptyAction = err else {
        panic!("expected EmptyAction error, got: {err:?}");
    };
}

#[test]
fn update_did_parse_invalid_previous_operation_hash() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: vec![1, 2, 3], // too short
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                proto::prism_ssi::RemoveKeyAction {
                    keyId: "key-0".to_string(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::InvalidPreviousOperationHash { .. } = err else {
        panic!("expected InvalidPreviousOperationHash error, got: {err:?}");
    };
}

#[test]
fn update_did_parse_invalid_did_suffix() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: "not-valid-hex!".to_string(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                proto::prism_ssi::RemoveKeyAction {
                    keyId: "key-0".to_string(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::InvalidDidSyntax { .. } = err else {
        panic!("expected InvalidDidSyntax error, got: {err:?}");
    };
}

#[test]
fn update_did_parse_success_remove_key() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                proto::prism_ssi::RemoveKeyAction {
                    keyId: "key-0".to_string(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let result = UpdateDidOperation::parse(&param, &proto).unwrap();
    assert_eq!(result.actions.len(), 1);
    let UpdateOperationAction::RemoveKey(kid) = &result.actions[0] else {
        panic!("expected RemoveKey action");
    };
    assert_eq!(kid.as_str(), "key-0");
}

#[test]
fn update_did_parse_add_key() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::AddKey(
                proto::prism_ssi::AddKeyAction {
                    key: Some(secp_key_proto(
                        "auth-1",
                        proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY,
                        5,
                    ))
                    .into(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let result = UpdateDidOperation::parse(&param, &proto).unwrap();
    assert_eq!(result.actions.len(), 1);
    let UpdateOperationAction::AddKey(pk) = &result.actions[0] else {
        panic!("expected AddKey action");
    };
    assert_eq!(pk.id.as_str(), "auth-1");
}

#[test]
fn update_did_parse_add_key_missing_key_data() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::AddKey(
                proto::prism_ssi::AddKeyAction {
                    key: None.into(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::MissingUpdateActionData {
        action_type,
        field_name,
    } = err
    else {
        panic!("expected MissingUpdateActionData error, got: {err:?}");
    };
    assert_eq!(field_name, "key");
    assert!(action_type.contains("AddKeyAction"));
}

#[test]
fn update_did_parse_add_service() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::AddService(
                proto::prism_ssi::AddServiceAction {
                    service: Some(service_proto("svc-0", "LinkedDomains", "https://example.com")).into(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let result = UpdateDidOperation::parse(&param, &proto).unwrap();
    assert_eq!(result.actions.len(), 1);
    let UpdateOperationAction::AddService(svc) = &result.actions[0] else {
        panic!("expected AddService action");
    };
    assert_eq!(svc.id.as_str(), "svc-0");
}

#[test]
fn update_did_parse_add_service_missing_service_data() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::AddService(
                proto::prism_ssi::AddServiceAction {
                    service: None.into(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::MissingUpdateActionData {
        action_type,
        field_name,
    } = err
    else {
        panic!("expected MissingUpdateActionData error, got: {err:?}");
    };
    assert_eq!(field_name, "service");
    assert!(action_type.contains("AddServiceAction"));
}

#[test]
fn update_did_parse_remove_service() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::RemoveService(
                proto::prism_ssi::RemoveServiceAction {
                    serviceId: "svc-0".to_string(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let result = UpdateDidOperation::parse(&param, &proto).unwrap();
    assert_eq!(result.actions.len(), 1);
    let UpdateOperationAction::RemoveService(sid) = &result.actions[0] else {
        panic!("expected RemoveService action");
    };
    assert_eq!(sid.as_str(), "svc-0");
}

#[test]
fn update_did_parse_update_service_with_type_and_endpoint() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                proto::prism_ssi::UpdateServiceAction {
                    serviceId: "svc-0".to_string(),
                    type_: "LinkedDomains".to_string(),
                    service_endpoints: "https://updated.example.com".to_string(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let result = UpdateDidOperation::parse(&param, &proto).unwrap();
    assert_eq!(result.actions.len(), 1);
    let UpdateOperationAction::UpdateService {
        id,
        r#type,
        service_endpoints,
    } = &result.actions[0]
    else {
        panic!("expected UpdateService action");
    };
    assert_eq!(id.as_str(), "svc-0");
    assert!(r#type.is_some());
    assert!(service_endpoints.is_some());
}

#[test]
fn update_did_parse_update_service_empty_type_and_endpoint() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                proto::prism_ssi::UpdateServiceAction {
                    serviceId: "svc-0".to_string(),
                    type_: String::new(),
                    service_endpoints: String::new(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let result = UpdateDidOperation::parse(&param, &proto).unwrap();
    let UpdateOperationAction::UpdateService {
        r#type,
        service_endpoints,
        ..
    } = &result.actions[0]
    else {
        panic!("expected UpdateService action");
    };
    assert!(r#type.is_none());
    assert!(service_endpoints.is_none());
}

#[test]
fn update_did_parse_patch_context() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::PatchContext(
                proto::prism_ssi::PatchContextAction {
                    context: vec!["https://custom.context/v1".to_string()],
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let result = UpdateDidOperation::parse(&param, &proto).unwrap();
    assert_eq!(result.actions.len(), 1);
    let UpdateOperationAction::PatchContext(ctx) = &result.actions[0] else {
        panic!("expected PatchContext action");
    };
    assert_eq!(ctx.len(), 1);
    assert_eq!(ctx[0], "https://custom.context/v1");
}

#[test]
fn update_did_parse_patch_context_duplicate() {
    let param = v1_params();
    let ctx = "https://custom.context/v1".to_string();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::PatchContext(
                proto::prism_ssi::PatchContextAction {
                    context: vec![ctx.clone(), ctx],
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::DuplicateContext = err else {
        panic!("expected DuplicateContext error, got: {err:?}");
    };
}

#[test]
fn update_did_parse_none_action_returns_none() {
    let param = v1_params();
    let action = proto::prism_ssi::UpdateDIDAction {
        action: None, // no action set
        special_fields: Default::default(),
    };

    let result = UpdateOperationAction::parse(&action, &param).unwrap();
    assert!(result.is_none());
}

// ============================================================================
// DeactivateDidOperation::parse tests
// ============================================================================

#[test]
fn deactivate_did_parse_success() {
    let proto = proto::prism_ssi::ProtoDeactivateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        special_fields: Default::default(),
    };

    let result = DeactivateDidOperation::parse(&proto).unwrap();
    assert!(result.id.to_string().ends_with(&valid_did_suffix()));
}

#[test]
fn deactivate_did_parse_invalid_prev_hash() {
    let proto = proto::prism_ssi::ProtoDeactivateDID {
        previous_operation_hash: vec![1, 2, 3], // too short
        id: valid_did_suffix(),
        special_fields: Default::default(),
    };

    let err = DeactivateDidOperation::parse(&proto).unwrap_err();
    let DeactivateDidOperationError::InvalidPreviousOperationHash { .. } = err else {
        panic!("expected InvalidPreviousOperationHash error, got: {err:?}");
    };
}

#[test]
fn deactivate_did_parse_invalid_did_suffix() {
    let proto = proto::prism_ssi::ProtoDeactivateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: "not-valid-hex!".to_string(),
        special_fields: Default::default(),
    };

    let err = DeactivateDidOperation::parse(&proto).unwrap_err();
    let DeactivateDidOperationError::InvalidDidSyntax { .. } = err else {
        panic!("expected InvalidDidSyntax error, got: {err:?}");
    };
}

// ============================================================================
// PublicKeyId::parse tests
// ============================================================================

#[test]
fn public_key_id_parse_valid() {
    let id = PublicKeyId::parse("master-0", 50).unwrap();
    assert_eq!(id.as_str(), "master-0");
}

#[test]
fn public_key_id_parse_empty() {
    let err = PublicKeyId::parse("", 50).unwrap_err();
    assert!(matches!(err, PublicKeyIdError::Empty));
}

#[test]
fn public_key_id_parse_too_long() {
    let long_id = "a".repeat(51);
    let err = PublicKeyId::parse(&long_id, 50).unwrap_err();
    assert!(matches!(err, PublicKeyIdError::TooLong { .. }));
}

#[test]
fn public_key_id_parse_invalid_uri_fragment() {
    let err = PublicKeyId::parse("has space", 50).unwrap_err();
    assert!(matches!(err, PublicKeyIdError::InvalidUriFragment));
}

#[test]
fn public_key_id_parse_display() {
    let id = PublicKeyId::parse("master-0", 50).unwrap();
    assert_eq!(format!("{id}"), "master-0");
}

// ============================================================================
// ServiceId::parse tests
// ============================================================================

#[test]
fn service_id_parse_valid() {
    let id = ServiceId::parse("svc-0", 50).unwrap();
    assert_eq!(id.as_str(), "svc-0");
}

#[test]
fn service_id_parse_empty() {
    let err = ServiceId::parse("", 50).unwrap_err();
    assert!(matches!(err, ServiceIdError::Empty));
}

#[test]
fn service_id_parse_too_long() {
    let long_id = "a".repeat(51);
    let err = ServiceId::parse(&long_id, 50).unwrap_err();
    assert!(matches!(err, ServiceIdError::TooLong { .. }));
}

#[test]
fn service_id_parse_invalid_uri_fragment() {
    let err = ServiceId::parse("has space", 50).unwrap_err();
    assert!(matches!(err, ServiceIdError::InvalidUriFragment));
}

#[test]
fn service_id_parse_display() {
    let id = ServiceId::parse("svc-0", 50).unwrap();
    assert_eq!(format!("{id}"), "svc-0");
}

// ============================================================================
// PublicKey::parse tests
// ============================================================================

#[test]
fn public_key_parse_master_key_secp256k1_compressed() {
    let param = v1_params();
    let pk_proto = master_key_proto("master-0");
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    assert_eq!(result.id.as_str(), "master-0");
    assert_eq!(result.data.usage(), KeyUsage::MasterKey);
    let PublicKeyData::Master { .. } = &result.data else {
        panic!("expected Master key data");
    };
}

#[test]
fn public_key_parse_master_key_secp256k1_uncompressed() {
    let param = v1_params();
    let pk_proto = secp_key_uncompressed_proto("master-0", proto::prism_ssi::KeyUsage::MASTER_KEY);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    assert_eq!(result.id.as_str(), "master-0");
    assert_eq!(result.data.usage(), KeyUsage::MasterKey);
}

#[test]
fn public_key_parse_vdr_key() {
    let param = v1_params();
    let pk_proto = secp_key_proto("vdr-0", proto::prism_ssi::KeyUsage::VDR_KEY, 2);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Vdr { .. } = &result.data else {
        panic!("expected Vdr key data");
    };
    assert_eq!(result.data.usage(), KeyUsage::VdrKey);
}

#[test]
fn public_key_parse_authentication_key() {
    let param = v1_params();
    let pk_proto = secp_key_proto("auth-0", proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY, 3);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { data: _, usage } = &result.data else {
        panic!("expected Other key data");
    };
    assert_eq!(*usage, KeyUsage::AuthenticationKey);
}

#[test]
fn public_key_parse_issuing_key() {
    let param = v1_params();
    let pk_proto = secp_key_proto("issue-0", proto::prism_ssi::KeyUsage::ISSUING_KEY, 4);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { usage, .. } = &result.data else {
        panic!("expected Other key data");
    };
    assert_eq!(*usage, KeyUsage::IssuingKey);
}

#[test]
fn public_key_parse_key_agreement_key() {
    let param = v1_params();
    let pk_proto = secp_key_proto("ka-0", proto::prism_ssi::KeyUsage::KEY_AGREEMENT_KEY, 5);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { usage, .. } = &result.data else {
        panic!("expected Other key data");
    };
    assert_eq!(*usage, KeyUsage::KeyAgreementKey);
}

#[test]
fn public_key_parse_revocation_key() {
    let param = v1_params();
    let pk_proto = secp_key_proto("rev-0", proto::prism_ssi::KeyUsage::REVOCATION_KEY, 6);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { usage, .. } = &result.data else {
        panic!("expected Other key data");
    };
    assert_eq!(*usage, KeyUsage::RevocationKey);
}

#[test]
fn public_key_parse_capability_invocation_key() {
    let param = v1_params();
    let pk_proto = secp_key_proto("ci-0", proto::prism_ssi::KeyUsage::CAPABILITY_INVOCATION_KEY, 7);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { usage, .. } = &result.data else {
        panic!("expected Other key data");
    };
    assert_eq!(*usage, KeyUsage::CapabilityInvocationKey);
}

#[test]
fn public_key_parse_capability_delegation_key() {
    let param = v1_params();
    let pk_proto = secp_key_proto("cd-0", proto::prism_ssi::KeyUsage::CAPABILITY_DELEGATION_KEY, 8);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { usage, .. } = &result.data else {
        panic!("expected Other key data");
    };
    assert_eq!(*usage, KeyUsage::CapabilityDelegationKey);
}

#[test]
fn public_key_parse_master_key_not_secp256k1() {
    let param = v1_params();
    let pk_proto = ed25519_key_proto("master-0", proto::prism_ssi::KeyUsage::MASTER_KEY);
    let err = PublicKey::parse(&pk_proto, &param).unwrap_err();
    let PublicKeyError::MasterKeyNotSecp256k1 { id } = err else {
        panic!("expected MasterKeyNotSecp256k1 error, got: {err:?}");
    };
    assert_eq!(id.as_str(), "master-0");
}

#[test]
fn public_key_parse_vdr_key_not_secp256k1() {
    let param = v1_params();
    let pk_proto = ed25519_key_proto("vdr-0", proto::prism_ssi::KeyUsage::VDR_KEY);
    let err = PublicKey::parse(&pk_proto, &param).unwrap_err();
    let PublicKeyError::VdrKeyNotSecp256k1 { id } = err else {
        panic!("expected VdrKeyNotSecp256k1 error, got: {err:?}");
    };
    assert_eq!(id.as_str(), "vdr-0");
}

#[test]
fn public_key_parse_missing_key_data() {
    let param = v1_params();
    let pk_proto = proto::prism_ssi::PublicKey {
        id: "master-0".to_string(),
        usage: proto::prism_ssi::KeyUsage::MASTER_KEY.into(),
        key_data: None,
        special_fields: Default::default(),
    };
    let err = PublicKey::parse(&pk_proto, &param).unwrap_err();
    let PublicKeyError::MissingKeyData { id } = err else {
        panic!("expected MissingKeyData error, got: {err:?}");
    };
    assert_eq!(id.as_str(), "master-0");
}

#[test]
fn public_key_parse_unknown_key_usage() {
    let param = v1_params();
    let pk_proto = proto::prism_ssi::PublicKey {
        id: "key-0".to_string(),
        usage: proto::prism_ssi::KeyUsage::UNKNOWN_KEY.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
            proto::prism_ssi::CompressedECKeyData {
                curve: "secp256k1".to_string(),
                data: vec![0u8; 33],
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let err = PublicKey::parse(&pk_proto, &param).unwrap_err();
    let PublicKeyError::UnknownKeyUsage { id } = err else {
        panic!("expected UnknownKeyUsage error, got: {err:?}");
    };
    assert_eq!(id.as_str(), "key-0");
}

#[test]
fn public_key_parse_unsupported_curve() {
    let param = v1_params();
    let pk_proto = proto::prism_ssi::PublicKey {
        id: "key-0".to_string(),
        usage: proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
            proto::prism_ssi::CompressedECKeyData {
                curve: "unknown-curve".to_string(),
                data: vec![0u8; 33],
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let err = PublicKey::parse(&pk_proto, &param).unwrap_err();
    let PublicKeyError::UnsupportedCurve { id } = err else {
        panic!("expected UnsupportedCurve error, got: {err:?}");
    };
    assert_eq!(id.as_str(), "key-0");
}

#[test]
fn public_key_parse_invalid_key_id() {
    let param = v1_params();
    let pk_proto = proto::prism_ssi::PublicKey {
        id: "".to_string(), // empty id
        usage: proto::prism_ssi::KeyUsage::MASTER_KEY.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
            proto::prism_ssi::CompressedECKeyData {
                curve: "secp256k1".to_string(),
                data: vec![0u8; 33],
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let err = PublicKey::parse(&pk_proto, &param).unwrap_err();
    let PublicKeyError::InvalidKeyId { id, .. } = err else {
        panic!("expected InvalidKeyId error, got: {err:?}");
    };
    assert_eq!(id, "");
}

#[test]
fn public_key_parse_ed25519_key() {
    let param = v1_params();
    let pk_proto = ed25519_key_proto("auth-0", proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { data, usage } = &result.data else {
        panic!("expected Other key data");
    };
    assert_eq!(*usage, KeyUsage::AuthenticationKey);
    let NonOperationPublicKey::Ed25519(_) = data else {
        panic!("expected Ed25519 key");
    };
}

#[test]
fn public_key_parse_x25519_key() {
    let param = v1_params();
    let pk_proto = x25519_key_proto("ka-0", proto::prism_ssi::KeyUsage::KEY_AGREEMENT_KEY);
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { data, usage } = &result.data else {
        panic!("expected Other key data");
    };
    assert_eq!(*usage, KeyUsage::KeyAgreementKey);
    let NonOperationPublicKey::X25519(_) = data else {
        panic!("expected X25519 key");
    };
}

#[test]
fn public_key_parse_ed25519_uncompressed() {
    let param = v1_params();
    let pk_bytes = [42u8; 32];
    let pk_proto = proto::prism_ssi::PublicKey {
        id: "auth-0".to_string(),
        usage: proto::prism_ssi::KeyUsage::AUTHENTICATION_KEY.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::EcKeyData(
            proto::prism_ssi::ECKeyData {
                curve: "Ed25519".to_string(),
                x: pk_bytes.into(),
                y: vec![], // Ed25519 doesn't use y but proto allows it
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { data, .. } = &result.data else {
        panic!("expected Other key data");
    };
    let NonOperationPublicKey::Ed25519(_) = data else {
        panic!("expected Ed25519 key");
    };
}

#[test]
fn public_key_parse_x25519_uncompressed() {
    let param = v1_params();
    let pk_bytes = [99u8; 32];
    let pk_proto = proto::prism_ssi::PublicKey {
        id: "ka-0".to_string(),
        usage: proto::prism_ssi::KeyUsage::KEY_AGREEMENT_KEY.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::EcKeyData(
            proto::prism_ssi::ECKeyData {
                curve: "X25519".to_string(),
                x: pk_bytes.into(),
                y: vec![],
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let result = PublicKey::parse(&pk_proto, &param).unwrap();
    let PublicKeyData::Other { data, .. } = &result.data else {
        panic!("expected Other key data");
    };
    let NonOperationPublicKey::X25519(_) = data else {
        panic!("expected X25519 key");
    };
}

// ============================================================================
// KeyUsage::parse tests
// ============================================================================

#[test]
fn key_usage_parse_all_variants() {
    use proto::prism_ssi::KeyUsage as ProtoKeyUsage;

    assert_eq!(KeyUsage::parse(&ProtoKeyUsage::MASTER_KEY), Some(KeyUsage::MasterKey));
    assert_eq!(KeyUsage::parse(&ProtoKeyUsage::ISSUING_KEY), Some(KeyUsage::IssuingKey));
    assert_eq!(
        KeyUsage::parse(&ProtoKeyUsage::KEY_AGREEMENT_KEY),
        Some(KeyUsage::KeyAgreementKey)
    );
    assert_eq!(
        KeyUsage::parse(&ProtoKeyUsage::AUTHENTICATION_KEY),
        Some(KeyUsage::AuthenticationKey)
    );
    assert_eq!(
        KeyUsage::parse(&ProtoKeyUsage::REVOCATION_KEY),
        Some(KeyUsage::RevocationKey)
    );
    assert_eq!(
        KeyUsage::parse(&ProtoKeyUsage::CAPABILITY_INVOCATION_KEY),
        Some(KeyUsage::CapabilityInvocationKey)
    );
    assert_eq!(
        KeyUsage::parse(&ProtoKeyUsage::CAPABILITY_DELEGATION_KEY),
        Some(KeyUsage::CapabilityDelegationKey)
    );
    assert_eq!(KeyUsage::parse(&ProtoKeyUsage::VDR_KEY), Some(KeyUsage::VdrKey));
    assert_eq!(KeyUsage::parse(&ProtoKeyUsage::UNKNOWN_KEY), None);
}

// ============================================================================
// PublicKeyData::usage tests
// ============================================================================

#[test]
fn public_key_data_usage_master() {
    let sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let pk = sk.to_public_key();
    let data = PublicKeyData::Master { data: pk };
    assert_eq!(data.usage(), KeyUsage::MasterKey);
}

#[test]
fn public_key_data_usage_vdr() {
    let sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let pk = sk.to_public_key();
    let data = PublicKeyData::Vdr { data: pk };
    assert_eq!(data.usage(), KeyUsage::VdrKey);
}

#[test]
fn public_key_data_usage_other() {
    let sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let pk = sk.to_public_key();
    let data = PublicKeyData::Other {
        data: NonOperationPublicKey::Secp256k1(pk),
        usage: KeyUsage::AuthenticationKey,
    };
    assert_eq!(data.usage(), KeyUsage::AuthenticationKey);
}

// ============================================================================
// ServiceType::parse tests
// ============================================================================

#[test]
fn service_type_parse_single_string() {
    let param = v1_params();
    let st = ServiceType::parse("LinkedDomains", &param).unwrap();
    let ServiceType::One(val) = st else {
        panic!("expected ServiceType::One");
    };
    assert_eq!(val.to_string(), "LinkedDomains");
}

#[test]
fn service_type_parse_json_list() {
    let param = v1_params();
    let json = r#"["LinkedDomains","Messaging"]"#;
    let st = ServiceType::parse(json, &param).unwrap();
    let ServiceType::Many(vals) = st else {
        panic!("expected ServiceType::Many");
    };
    assert_eq!(vals.len(), 2);
}

#[test]
fn service_type_parse_exceed_max_size() {
    let mut param = v1_params();
    param.max_type_size = 5;
    let err = ServiceType::parse("LinkedDomains", &param).unwrap_err();
    assert!(matches!(err, ServiceTypeError::ExceedMaxSize { .. }));
}

#[test]
fn service_type_parse_empty_json_list() {
    let param = v1_params();
    let err = ServiceType::parse("[]", &param).unwrap_err();
    assert!(matches!(err, ServiceTypeError::Empty));
}

#[test]
fn service_type_parse_invalid_json_list_syntax() {
    let param = v1_params();
    // This is a valid JSON array but the re-serialization won't match (extra whitespace/formatting)
    // Actually, serde_json normalizes: "[\"a\"]" -> ["a"] which does match. We need a case that doesn't roundtrip.
    // Using a single string that is valid JSON array but with different ordering or spacing
    let json = "[ \"LinkedDomains\" ]"; // extra spaces
    let err = ServiceType::parse(json, &param).unwrap_err();
    assert!(matches!(err, ServiceTypeError::InvalidSyntax));
}

#[test]
fn service_type_parse_invalid_name() {
    let param = v1_params();
    let err = ServiceType::parse("Invalid!Type", &param).unwrap_err();
    assert!(matches!(err, ServiceTypeError::InvalidSyntax));
}

#[test]
fn service_type_value_from_str_valid() {
    let val = ServiceTypeValue::from_str("LinkedDomains").unwrap();
    assert_eq!(val.to_string(), "LinkedDomains");
}

#[test]
fn service_type_value_from_str_with_hyphen() {
    let val = ServiceTypeValue::from_str("Linked-Domains").unwrap();
    assert_eq!(val.to_string(), "Linked-Domains");
}

#[test]
fn service_type_value_from_str_with_underscore() {
    let val = ServiceTypeValue::from_str("Linked_Domains").unwrap();
    assert_eq!(val.to_string(), "Linked_Domains");
}

#[test]
fn service_type_value_from_str_invalid() {
    let err = ServiceTypeValue::from_str("Invalid!").unwrap_err();
    assert!(matches!(err, ServiceTypeError::InvalidSyntax));
}

// ============================================================================
// ServiceEndpoint::parse tests
// ============================================================================

#[test]
fn service_endpoint_parse_uri() {
    let param = v1_params();
    let ep = ServiceEndpoint::parse("https://example.com", &param).unwrap();
    let ServiceEndpoint::One(ServiceEndpointValue::Uri(uri)) = ep else {
        panic!("expected URI endpoint");
    };
    assert_eq!(uri, "https://example.com");
}

#[test]
fn service_endpoint_parse_json_object() {
    let param = v1_params();
    let json = r#"{"key":"value"}"#;
    let ep = ServiceEndpoint::parse(json, &param).unwrap();
    let ServiceEndpoint::One(ServiceEndpointValue::Json(map)) = ep else {
        panic!("expected JSON endpoint");
    };
    assert_eq!(map.get("key").unwrap().as_str().unwrap(), "value");
}

#[test]
fn service_endpoint_parse_json_array_of_uris() {
    let param = v1_params();
    let json = r#"["https://a.com","https://b.com"]"#;
    let ep = ServiceEndpoint::parse(json, &param).unwrap();
    let ServiceEndpoint::Many(vals) = ep else {
        panic!("expected Many endpoint");
    };
    assert_eq!(vals.len(), 2);
}

#[test]
fn service_endpoint_parse_json_array_with_objects() {
    let param = v1_params();
    let json = r#"[{"key":"val"}]"#;
    let ep = ServiceEndpoint::parse(json, &param).unwrap();
    let ServiceEndpoint::Many(vals) = ep else {
        panic!("expected Many endpoint");
    };
    assert_eq!(vals.len(), 1);
    let ServiceEndpointValue::Json(map) = &vals[0] else {
        panic!("expected JSON value");
    };
    assert_eq!(map.get("key").unwrap().as_str().unwrap(), "val");
}

#[test]
fn service_endpoint_parse_empty_json_array() {
    let param = v1_params();
    let err = ServiceEndpoint::parse("[]", &param).unwrap_err();
    assert!(matches!(err, ServiceEndpointError::Empty));
}

#[test]
fn service_endpoint_parse_exceed_max_size() {
    let mut param = v1_params();
    param.max_service_endpoint_size = 5;
    let err = ServiceEndpoint::parse("https://example.com", &param).unwrap_err();
    assert!(matches!(err, ServiceEndpointError::ExceedMaxSize { .. }));
}

#[test]
fn service_endpoint_parse_invalid_string() {
    let param = v1_params();
    let err = ServiceEndpoint::parse("not a valid uri", &param).unwrap_err();
    assert!(matches!(err, ServiceEndpointError::InvalidSyntax));
}

#[test]
fn service_endpoint_parse_json_array_with_invalid_element() {
    let param = v1_params();
    let json = r#"[42]"#; // number is invalid
    let err = ServiceEndpoint::parse(json, &param).unwrap_err();
    assert!(matches!(err, ServiceEndpointError::InvalidSyntax));
}

#[test]
fn service_endpoint_value_from_str_valid_uri() {
    let val = ServiceEndpointValue::from_str("https://example.com").unwrap();
    assert!(matches!(val, ServiceEndpointValue::Uri(s) if s == "https://example.com"));
}

#[test]
fn service_endpoint_value_from_str_invalid() {
    let err = ServiceEndpointValue::from_str("not a uri").unwrap_err();
    assert!(matches!(err, ServiceEndpointError::InvalidSyntax));
}

#[test]
fn service_endpoint_value_try_from_string() {
    let val = ServiceEndpointValue::try_from(serde_json::Value::String("https://example.com".to_string())).unwrap();
    assert!(matches!(val, ServiceEndpointValue::Uri(_)));
}

#[test]
fn service_endpoint_value_try_from_object() {
    let obj = serde_json::json!({"key": "value"});
    let val = ServiceEndpointValue::try_from(obj).unwrap();
    assert!(matches!(val, ServiceEndpointValue::Json(_)));
}

#[test]
fn service_endpoint_value_try_from_number() {
    let val = serde_json::json!(42);
    let err = ServiceEndpointValue::try_from(val).unwrap_err();
    assert!(matches!(err, ServiceEndpointError::InvalidSyntax));
}

// ============================================================================
// Service::parse tests
// ============================================================================

#[test]
fn service_parse_success() {
    let param = v1_params();
    let svc_proto = service_proto("svc-0", "LinkedDomains", "https://example.com");
    let svc = Service::parse(&svc_proto, &param).unwrap();
    assert_eq!(svc.id.as_str(), "svc-0");
}

#[test]
fn service_parse_invalid_id() {
    let param = v1_params();
    let svc_proto = proto::prism_ssi::Service {
        id: "".to_string(),
        type_: "LinkedDomains".to_string(),
        service_endpoint: "https://example.com".to_string(),
        special_fields: Default::default(),
    };
    let err = Service::parse(&svc_proto, &param).unwrap_err();
    assert!(matches!(err, ServiceError::InvalidServiceId { .. }));
}

#[test]
fn service_parse_invalid_type() {
    let param = v1_params();
    let svc_proto = proto::prism_ssi::Service {
        id: "svc-0".to_string(),
        type_: "Invalid!Type".to_string(),
        service_endpoint: "https://example.com".to_string(),
        special_fields: Default::default(),
    };
    let err = Service::parse(&svc_proto, &param).unwrap_err();
    assert!(matches!(err, ServiceError::InvalidServiceType { .. }));
}

#[test]
fn service_parse_invalid_endpoint() {
    let param = v1_params();
    let svc_proto = proto::prism_ssi::Service {
        id: "svc-0".to_string(),
        type_: "LinkedDomains".to_string(),
        service_endpoint: "not a uri".to_string(),
        special_fields: Default::default(),
    };
    let err = Service::parse(&svc_proto, &param).unwrap_err();
    assert!(matches!(err, ServiceError::InvalidServiceEndpoint { .. }));
}

// ============================================================================
// Service::update_type / update_service_endpoint tests
// ============================================================================

#[test]
fn service_update_type_single() {
    let param = v1_params();
    let svc_proto = service_proto("svc-0", "LinkedDomains", "https://example.com");
    let mut svc = Service::parse(&svc_proto, &param).unwrap();

    let new_type = ServiceType::One(ServiceTypeValue::from_str("Messaging").unwrap());
    svc.update_type(new_type);

    assert_eq!(svc.orig.type_, "Messaging");
    let ServiceType::One(val) = &svc.r#type else {
        panic!("expected ServiceType::One");
    };
    assert_eq!(val.to_string(), "Messaging");
}

#[test]
fn service_update_type_many() {
    let param = v1_params();
    let svc_proto = service_proto("svc-0", "LinkedDomains", "https://example.com");
    let mut svc = Service::parse(&svc_proto, &param).unwrap();

    let new_type = ServiceType::Many(vec![
        ServiceTypeValue::from_str("LinkedDomains").unwrap(),
        ServiceTypeValue::from_str("Messaging").unwrap(),
    ]);
    svc.update_type(new_type);

    // orig.type_ should be JSON array string
    let parsed: Vec<String> = serde_json::from_str(&svc.orig.type_).unwrap();
    assert_eq!(parsed.len(), 2);
}

#[test]
fn service_update_endpoint_uri() {
    let param = v1_params();
    let svc_proto = service_proto("svc-0", "LinkedDomains", "https://example.com");
    let mut svc = Service::parse(&svc_proto, &param).unwrap();

    let new_endpoint = ServiceEndpoint::One(ServiceEndpointValue::Uri("https://updated.com".to_string()));
    svc.update_service_endpoint(new_endpoint);

    assert_eq!(svc.orig.service_endpoint, "https://updated.com");
}

#[test]
fn service_update_endpoint_json_object() {
    let param = v1_params();
    let svc_proto = service_proto("svc-0", "LinkedDomains", "https://example.com");
    let mut svc = Service::parse(&svc_proto, &param).unwrap();

    let map = serde_json::json!({"key": "value"}).as_object().unwrap().clone();
    let new_endpoint = ServiceEndpoint::One(ServiceEndpointValue::Json(map));
    svc.update_service_endpoint(new_endpoint);

    // Should be JSON object string
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&svc.orig.service_endpoint).unwrap();
    assert_eq!(parsed.get("key").unwrap().as_str().unwrap(), "value");
}

#[test]
fn service_update_endpoint_many() {
    let param = v1_params();
    let svc_proto = service_proto("svc-0", "LinkedDomains", "https://example.com");
    let mut svc = Service::parse(&svc_proto, &param).unwrap();

    let new_endpoint = ServiceEndpoint::Many(vec![
        ServiceEndpointValue::Uri("https://a.com".to_string()),
        ServiceEndpointValue::Json(serde_json::json!({"key": "val"}).as_object().unwrap().clone()),
    ]);
    svc.update_service_endpoint(new_endpoint);

    let parsed: Vec<serde_json::Value> = serde_json::from_str(&svc.orig.service_endpoint).unwrap();
    assert_eq!(parsed.len(), 2);
}

// ============================================================================
// NonOperationPublicKey parse edge cases
// ============================================================================

#[test]
fn non_op_public_key_parse_secp256k1_uncompressed() {
    let sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let pk = sk.to_public_key();
    let uncompressed = pk.encode_uncompressed();
    let x = &uncompressed[1..33];
    let y = &uncompressed[33..65];

    let key_data = proto::prism_ssi::public_key::Key_data::EcKeyData(proto::prism_ssi::ECKeyData {
        curve: "secp256k1".to_string(),
        x: x.into(),
        y: y.into(),
        special_fields: Default::default(),
    });

    let result = NonOperationPublicKey::parse(&key_data).unwrap().unwrap();
    let NonOperationPublicKey::Secp256k1(parsed_pk) = result else {
        panic!("expected Secp256k1");
    };
    assert_eq!(parsed_pk, pk);
}

#[test]
fn non_op_public_key_parse_unknown_curve() {
    let key_data = proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(proto::prism_ssi::CompressedECKeyData {
        curve: "unknown-curve".to_string(),
        data: vec![0u8; 33],
        special_fields: Default::default(),
    });

    let result = NonOperationPublicKey::parse(&key_data).unwrap();
    assert!(result.is_none());
}

#[test]
fn non_op_public_key_parse_secp256k1_bad_data() {
    let key_data = proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(proto::prism_ssi::CompressedECKeyData {
        curve: "secp256k1".to_string(),
        data: vec![0u8; 10], // too short for secp256k1 compressed
        special_fields: Default::default(),
    });

    let result = NonOperationPublicKey::parse(&key_data);
    assert!(result.is_err());
}

#[test]
fn non_op_public_key_parse_ed25519_bad_data() {
    let key_data = proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(proto::prism_ssi::CompressedECKeyData {
        curve: "Ed25519".to_string(),
        data: vec![0u8; 10], // too short
        special_fields: Default::default(),
    });

    let result = NonOperationPublicKey::parse(&key_data);
    assert!(result.is_err());
}

#[test]
fn non_op_public_key_parse_x25519_bad_data() {
    let key_data = proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(proto::prism_ssi::CompressedECKeyData {
        curve: "X25519".to_string(),
        data: vec![0u8; 10], // too short
        special_fields: Default::default(),
    });

    let result = NonOperationPublicKey::parse(&key_data);
    assert!(result.is_err());
}

#[test]
fn non_op_public_key_equality() {
    let sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let pk = sk.to_public_key();
    let a = NonOperationPublicKey::Secp256k1(pk.clone());
    let b = NonOperationPublicKey::Secp256k1(pk);
    assert_eq!(a, b);
}

#[test]
fn non_op_public_key_encode_jwk_secp256k1() {
    let sk = Secp256k1PrivateKey::from_slice(&[1u8; 32]).unwrap();
    let pk = sk.to_public_key();
    let nopk = NonOperationPublicKey::Secp256k1(pk);
    let jwk = nopk.encode_jwk();
    assert_eq!(jwk.kty, "EC");
    assert_eq!(jwk.crv, "secp256k1");
}

#[test]
fn non_op_public_key_encode_jwk_ed25519() {
    let pk = identus_apollo::crypto::ed25519::Ed25519PublicKey::from_slice(&[42u8; 32]).unwrap();
    let nopk = NonOperationPublicKey::Ed25519(pk);
    let jwk = nopk.encode_jwk();
    assert_eq!(jwk.kty, "OKP");
    assert_eq!(jwk.crv, "Ed25519");
}

#[test]
fn non_op_public_key_encode_jwk_x25519() {
    let pk = identus_apollo::crypto::x25519::X25519PublicKey::from_slice(&[99u8; 32]).unwrap();
    let nopk = NonOperationPublicKey::X25519(pk);
    let jwk = nopk.encode_jwk();
    assert_eq!(jwk.kty, "OKP");
    assert_eq!(jwk.crv, "X25519");
}

// ============================================================================
// Additional tests for uncovered error paths in UpdateOperationAction::parse
// ============================================================================

#[test]
fn update_did_parse_remove_key_with_invalid_key_id() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::RemoveKey(
                proto::prism_ssi::RemoveKeyAction {
                    keyId: "has invalid space".to_string(), // not a valid URI fragment
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::InvalidPublicKey {
        source: PublicKeyError::InvalidKeyId { id, .. },
    } = err
    else {
        panic!("expected InvalidPublicKey/InvalidKeyId error, got: {err:?}");
    };
    assert_eq!(id, "has invalid space");
}

#[test]
fn update_did_parse_remove_service_with_invalid_service_id() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::RemoveService(
                proto::prism_ssi::RemoveServiceAction {
                    serviceId: "has invalid space".to_string(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::InvalidService {
        source: ServiceError::InvalidServiceId { id, .. },
    } = err
    else {
        panic!("expected InvalidService/InvalidServiceId error, got: {err:?}");
    };
    assert_eq!(id, "has invalid space");
}

#[test]
fn update_did_parse_update_service_with_invalid_service_id() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                proto::prism_ssi::UpdateServiceAction {
                    serviceId: "has invalid space".to_string(),
                    type_: "LinkedDomains".to_string(),
                    service_endpoints: "https://example.com".to_string(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::InvalidService {
        source: ServiceError::InvalidServiceId { id, .. },
    } = err
    else {
        panic!("expected InvalidService/InvalidServiceId error, got: {err:?}");
    };
    assert_eq!(id, "has invalid space");
}

#[test]
fn update_did_parse_update_service_with_invalid_type() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                proto::prism_ssi::UpdateServiceAction {
                    serviceId: "svc-0".to_string(),
                    type_: "Invalid!Type".to_string(), // invalid service type name
                    service_endpoints: "https://example.com".to_string(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::InvalidService {
        source: ServiceError::InvalidServiceType { type_name, .. },
    } = err
    else {
        panic!("expected InvalidService/InvalidServiceType error, got: {err:?}");
    };
    assert_eq!(type_name, "Invalid!Type");
}

#[test]
fn update_did_parse_update_service_with_invalid_endpoint() {
    let param = v1_params();
    let proto = proto::prism_ssi::ProtoUpdateDID {
        previous_operation_hash: valid_prev_op_hash(),
        id: valid_did_suffix(),
        actions: vec![proto::prism_ssi::UpdateDIDAction {
            action: Some(proto::prism_ssi::update_didaction::Action::UpdateService(
                proto::prism_ssi::UpdateServiceAction {
                    serviceId: "svc-0".to_string(),
                    type_: "LinkedDomains".to_string(),
                    service_endpoints: "not a valid uri".to_string(), // invalid endpoint
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        }],
        special_fields: Default::default(),
    };

    let err = UpdateDidOperation::parse(&param, &proto).unwrap_err();
    let UpdateDidOperationError::InvalidService {
        source: ServiceError::InvalidServiceEndpoint { endpoint, .. },
    } = err
    else {
        panic!("expected InvalidService/InvalidServiceEndpoint error, got: {err:?}");
    };
    assert_eq!(endpoint, "not a valid uri");
}

// ============================================================================
// PublicKey::parse InvalidKeyData error path
// ============================================================================

#[test]
fn public_key_parse_invalid_key_data_secp256k1() {
    let param = v1_params();
    // Provide secp256k1 curve with invalid compressed data (wrong length)
    let pk_proto = proto::prism_ssi::PublicKey {
        id: "master-0".to_string(),
        usage: proto::prism_ssi::KeyUsage::MASTER_KEY.into(),
        key_data: Some(proto::prism_ssi::public_key::Key_data::CompressedEcKeyData(
            proto::prism_ssi::CompressedECKeyData {
                curve: "secp256k1".to_string(),
                data: vec![0x02; 10], // too short for secp256k1 compressed key (needs 33 bytes)
                special_fields: Default::default(),
            },
        )),
        special_fields: Default::default(),
    };
    let err = PublicKey::parse(&pk_proto, &param).unwrap_err();
    let PublicKeyError::InvalidKeyData { id, .. } = err else {
        panic!("expected InvalidKeyData error, got: {err:?}");
    };
    assert_eq!(id.as_str(), "master-0");
}
