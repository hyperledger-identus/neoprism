pub mod did;
pub mod dlt;
pub mod error;
mod macros;
pub mod prelude;
pub mod protocol;
pub mod utils;

#[allow(clippy::doc_lazy_continuation)]
pub mod proto {
    use identus_apollo::hash::{Sha256Digest, sha256};
    use protobuf::Message;

    use crate::did::operation::OperationId;
    use crate::proto::prism::{PrismOperation, SignedPrismOperation};

    include!(concat!(env!("OUT_DIR"), "/generated/mod.rs"));

    pub trait MessageExt: Sized {
        fn encode_to_vec(&self) -> Vec<u8>;
        fn decode(bytes: &[u8]) -> protobuf::Result<Self>;
    }

    impl<T: Message> MessageExt for T {
        fn encode_to_vec(&self) -> Vec<u8> {
            self.write_to_bytes().expect("Unable to encode protobuf message to vec")
        }

        fn decode(bytes: &[u8]) -> protobuf::Result<Self> {
            Self::parse_from_bytes(bytes)
        }
    }

    impl PrismOperation {
        pub fn operation_hash(&self) -> Sha256Digest {
            let bytes = self.encode_to_vec();
            sha256(bytes)
        }
    }

    impl SignedPrismOperation {
        pub fn operation_hash(&self) -> Option<Sha256Digest> {
            self.operation.as_ref().map(|op| op.operation_hash())
        }

        /// Compute the operation ID by hashing the entire signed operation
        /// (including signature) as protobuf bytes
        pub fn operation_id(&self) -> OperationId {
            let bytes = self.encode_to_vec();
            sha256(bytes).into()
        }
    }
}
