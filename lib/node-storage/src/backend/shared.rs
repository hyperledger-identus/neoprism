use identus_did_prism::dlt::{BlockMetadata, OperationMetadata, TxId};
use identus_did_prism::prelude::*;
use identus_did_prism::proto::prism::SignedPrismOperation;
use identus_did_prism_indexer::repo::RawOperationId;

use crate::{Error, entity};

pub fn parse_raw_operation(
    value: entity::RawOperation,
) -> Result<(RawOperationId, OperationMetadata, SignedPrismOperation), Error> {
    let metadata = OperationMetadata {
        block_metadata: value.block_metadata()?,
        osn: value.osn.try_into().expect("osn value does not fit in u32"),
    };
    SignedPrismOperation::decode(value.signed_operation_data.as_slice())
        .map(|op| (value.id.into(), metadata, op))
        .map_err(|e| Error::ProtobufDecode {
            source: e,
            target_type: std::any::type_name::<SignedPrismOperation>(),
        })
}

impl entity::RawOperation {
    fn block_metadata(&self) -> Result<BlockMetadata, Error> {
        let tx_id = TxId::from_bytes(&self.tx_hash).expect("invalid tx_hash in database");
        Ok(BlockMetadata {
            slot_number: u64::try_from(self.slot).expect("slot value does not fit in u64").into(),
            block_number: u64::try_from(self.block_number)
                .expect("block_number value does not fit in u64")
                .into(),
            cbt: self.cbt,
            absn: self.absn.try_into().expect("absn value does not fit in u32"),
            tx_id,
        })
    }
}
