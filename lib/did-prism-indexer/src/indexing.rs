use identus_apollo::hash::Sha256Digest;
use identus_apollo::hex::HexStr;
use identus_did_prism::did::CanonicalPrismDid;
use identus_did_prism::dlt::OperationMetadata;
use identus_did_prism::prelude::*;
use identus_did_prism::proto::prism::prism_operation::Operation;

use crate::DltSource;
use crate::repo::{IndexedOperation, IndexedOperationRepo, RawOperationRepo};

enum IntermediateIndexedOperation {
    Ssi {
        did: CanonicalPrismDid,
    },
    VdrRoot {
        operation_hash: Vec<u8>,
        did: CanonicalPrismDid,
    },
    VdrChild {
        operation_hash: Vec<u8>,
        prev_operation_hash: Vec<u8>,
    },
}

/// Run indexer loop until no more operation to index
pub async fn run_indexer_loop<Repo>(repo: &Repo) -> anyhow::Result<()>
where
    Repo: RawOperationRepo + IndexedOperationRepo + ?Sized,
    <Repo as RawOperationRepo>::Error: Send + Sync + 'static,
    <Repo as IndexedOperationRepo>::Error: Send + Sync + 'static,
{
    loop {
        let unindexed_operations = repo.get_raw_operations_unindexed().await?;
        if unindexed_operations.is_empty() {
            return Ok(());
        }

        tracing::info!("Indexing {} operations", unindexed_operations.len());
        for (raw_operation_id, meta, signed_operation) in unindexed_operations {
            let intermediate_indexed_op = index_from_signed_operation(signed_operation);
            let indexed_op = match intermediate_indexed_op {
                Ok(IntermediateIndexedOperation::Ssi { did }) => IndexedOperation::Ssi { raw_operation_id, did },
                Ok(IntermediateIndexedOperation::VdrRoot { operation_hash, did }) => IndexedOperation::Vdr {
                    raw_operation_id,
                    init_operation_hash: operation_hash.clone(),
                    operation_hash,
                    did,
                    prev_operation_hash: None,
                },
                Ok(IntermediateIndexedOperation::VdrChild {
                    prev_operation_hash,
                    operation_hash,
                }) => {
                    let vdr_root = recursively_find_vdr_root(repo, &prev_operation_hash).await?;
                    match vdr_root {
                        Some((did, init_operation_hash)) => IndexedOperation::Vdr {
                            raw_operation_id,
                            init_operation_hash,
                            operation_hash,
                            prev_operation_hash: Some(prev_operation_hash),
                            did,
                        },
                        None => {
                            tracing::warn!("SignedPrismOperation {:?} is ignored since it cannot be indexed.", meta);
                            IndexedOperation::Ignored { raw_operation_id }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "SignedPrismOperation {:?} is ignored since it cannot be indexed. ({})",
                        meta,
                        e
                    );
                    IndexedOperation::Ignored { raw_operation_id }
                }
            };
            repo.insert_indexed_operations(vec![indexed_op]).await?;
        }
    }
}

/// Run sync loop until DLT source is closed
pub async fn run_sync_loop<Repo, Src>(repo: &Repo, source: Src) -> anyhow::Result<()>
where
    Src: DltSource,
    Repo: RawOperationRepo + IndexedOperationRepo + Send + Sync + ?Sized,
    <Repo as RawOperationRepo>::Error: Send + Sync + 'static,
    <Repo as IndexedOperationRepo>::Error: Send + Sync + 'static,
{
    let mut rx = source.into_stream().expect("Unable to create a DLT source");

    while let Some(published_prism_object) = rx.recv().await {
        let block = published_prism_object.prism_object.block_content;
        let block_metadata = published_prism_object.block_metadata;
        let signed_operations = block.map(|i| i.operations).unwrap_or_default();

        let mut insert_batch = Vec::with_capacity(signed_operations.len());
        for (idx, signed_operation) in signed_operations.into_iter().enumerate() {
            let has_operation = signed_operation
                .operation
                .as_ref()
                .and_then(|i| i.operation.as_ref())
                .is_some();

            if !has_operation {
                continue;
            }

            insert_batch.push((
                OperationMetadata {
                    block_metadata: block_metadata.clone(),
                    osn: idx as u32,
                },
                signed_operation,
            ));
        }

        let insert_result = repo.insert_raw_operations(insert_batch).await;
        if let Err(e) = insert_result {
            tracing::error!("Failed to insert operation into database: {:?}", e);
        }
    }
    Ok(())
}

/// Returns DID that create a root operation and its operation hash
async fn recursively_find_vdr_root<Repo>(
    repo: &Repo,
    prev_operation_hash: &[u8],
) -> anyhow::Result<Option<(CanonicalPrismDid, Vec<u8>)>>
where
    Repo: RawOperationRepo + IndexedOperationRepo + ?Sized,
    <Repo as RawOperationRepo>::Error: Send + Sync + 'static,
    <Repo as IndexedOperationRepo>::Error: Send + Sync + 'static,
{
    const SEARCH_MAX_DEPTH: usize = 200;
    let mut parent_hash = prev_operation_hash.to_vec();
    for _ in 1..SEARCH_MAX_DEPTH {
        let Ok(parsed_parent_hash) = Sha256Digest::from_bytes(&parent_hash) else {
            return Ok(None); // invalid parent
        };

        let parent = repo
            .get_raw_operation_vdr_by_operation_hash(&parsed_parent_hash)
            .await?;
        match parent {
            None => return Ok(None), // no root found
            Some((_, _, signed_operation)) => {
                match index_from_signed_operation(signed_operation) {
                    Ok(IntermediateIndexedOperation::VdrRoot { did, operation_hash }) => {
                        return Ok(Some((did, operation_hash))); // found root
                    }
                    Ok(IntermediateIndexedOperation::VdrChild {
                        prev_operation_hash, ..
                    }) => {
                        parent_hash = prev_operation_hash; // go to next parent
                    }
                    _ => return Ok(None), // invalid parent
                }
            }
        }
    }

    Ok(None) // exceed max depth
}

fn index_from_signed_operation(signed_operation: SignedPrismOperation) -> anyhow::Result<IntermediateIndexedOperation> {
    match signed_operation.operation.into_option() {
        Some(operation) => index_from_operation(operation),
        None => Err(anyhow::anyhow!("operation does not exist in PrismOperation")),
    }
}

fn index_from_operation(prism_operation: PrismOperation) -> anyhow::Result<IntermediateIndexedOperation> {
    let operation_hash = prism_operation.operation_hash();
    match prism_operation.operation {
        Some(Operation::CreateDid(_)) => Ok(IntermediateIndexedOperation::Ssi {
            did: CanonicalPrismDid::from_operation(&prism_operation)?,
        }),
        Some(Operation::UpdateDid(op)) => Ok(IntermediateIndexedOperation::Ssi {
            did: CanonicalPrismDid::from_suffix_str(&op.id)?,
        }),
        Some(Operation::DeactivateDid(op)) => Ok(IntermediateIndexedOperation::Ssi {
            did: CanonicalPrismDid::from_suffix_str(&op.id)?,
        }),
        Some(Operation::ProtocolVersionUpdate(op)) => Ok(IntermediateIndexedOperation::Ssi {
            did: CanonicalPrismDid::from_suffix_str(&op.proposer_did)?,
        }),
        Some(Operation::CreateStorageEntry(op)) => Ok(IntermediateIndexedOperation::VdrRoot {
            operation_hash: operation_hash.to_vec(),
            did: CanonicalPrismDid::from_suffix(HexStr::from(op.did_prism_hash.as_slice()))?,
        }),
        Some(Operation::UpdateStorageEntry(op)) => Ok(IntermediateIndexedOperation::VdrChild {
            operation_hash: operation_hash.to_vec(),
            prev_operation_hash: op.previous_event_hash,
        }),
        Some(Operation::DeactivateStorageEntry(op)) => Ok(IntermediateIndexedOperation::VdrChild {
            operation_hash: operation_hash.to_vec(),
            prev_operation_hash: op.previous_event_hash,
        }),
        None => Err(anyhow::anyhow!("operation does not exist in PrismOperation")),
        Some(_) => {
            let operation_hash_hex = HexStr::from(operation_hash.as_bytes());
            Err(anyhow::anyhow!(
                "operation type in PrismOperation is not support (operation_hash: {operation_hash_hex})"
            ))
        }
    }
}
