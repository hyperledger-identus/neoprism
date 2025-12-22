#![allow(unused)]

use chrono::DateTime;
use identus_apollo::crypto::secp256k1::Secp256k1PrivateKey;
use identus_apollo::hash::Sha256Digest;
use identus_did_prism::dlt::{BlockMetadata, OperationMetadata, TxId};
use identus_did_prism::prelude::*;
use identus_did_prism::proto;

const MASTER_KEY: [u8; 32] = [1; 32];
const MASTER_KEY_NAME: &str = "master-0";

#[derive(Default)]
pub struct CreateDidOptions {
    pub contexts: Option<Vec<String>>,
    pub public_keys: Option<Vec<proto::prism_ssi::PublicKey>>,
    pub services: Option<Vec<proto::prism_ssi::Service>>,
}

pub fn new_create_did_operation(
    options: Option<CreateDidOptions>,
) -> (proto::prism::SignedPrismOperation, Sha256Digest, Secp256k1PrivateKey) {
    let options = options.unwrap_or_default();
    let master_sk = Secp256k1PrivateKey::from_slice(&MASTER_KEY).unwrap();
    let mut public_keys = vec![new_public_key(
        MASTER_KEY_NAME,
        proto::prism_ssi::KeyUsage::MASTER_KEY,
        &master_sk,
    )];
    public_keys.extend_from_slice(&options.public_keys.unwrap_or_default());
    let operation_inner = proto::prism::prism_operation::Operation::CreateDid(proto::prism_ssi::ProtoCreateDID {
        did_data: Some(proto::prism_ssi::proto_create_did::DIDCreationData {
            public_keys,
            services: options.services.unwrap_or_default(),
            context: options.contexts.unwrap_or_default(),
            special_fields: Default::default(),
        })
        .into(),
        special_fields: Default::default(),
    });
    let operation = proto::prism::PrismOperation {
        operation: Some(operation_inner),
        special_fields: Default::default(),
    };
    let operation_hash = operation.operation_hash();
    let signed_operation = proto::prism::SignedPrismOperation {
        signed_with: MASTER_KEY_NAME.to_string(),
        signature: master_sk.sign(&operation.encode_to_vec()),
        operation: Some(operation).into(),
        special_fields: Default::default(),
    };
    (signed_operation, operation_hash, master_sk)
}

pub fn new_signed_operation(
    signed_with: &str,
    signing_key: &Secp256k1PrivateKey,
    operation: proto::prism::prism_operation::Operation,
) -> (proto::prism::SignedPrismOperation, Sha256Digest) {
    let operation = proto::prism::PrismOperation {
        operation: Some(operation),
        special_fields: Default::default(),
    };
    let operation_hash = operation.operation_hash();
    let signed_operation = proto::prism::SignedPrismOperation {
        signed_with: signed_with.to_string(),
        signature: signing_key.sign(&operation.encode_to_vec()),
        operation: Some(operation).into(),
        special_fields: Default::default(),
    };
    (signed_operation, operation_hash)
}

pub fn new_public_key(
    id: &str,
    usage: proto::prism_ssi::KeyUsage,
    sk: &Secp256k1PrivateKey,
) -> proto::prism_ssi::PublicKey {
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

pub fn populate_metadata(
    operations: Vec<proto::prism::SignedPrismOperation>,
) -> Vec<(OperationMetadata, proto::prism::SignedPrismOperation)> {
    let dummy_metadata = OperationMetadata {
        block_metadata: BlockMetadata {
            slot_number: 0.into(),
            block_number: 0.into(),
            cbt: DateTime::UNIX_EPOCH,
            absn: 0,
            tx_id: TxId::from(identus_apollo::hash::sha256([0u8; 32])),
        },
        osn: 0,
    };
    operations
        .into_iter()
        .enumerate()
        .map(|(idx, op)| {
            let metadata = OperationMetadata {
                osn: idx as u32,
                ..dummy_metadata.clone()
            };
            (metadata, op)
        })
        .collect()
}
