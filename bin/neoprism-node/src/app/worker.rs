use std::sync::Arc;
use std::time::Duration;

use identus_did_prism::dlt::DltCursor;
use identus_did_prism_indexer::{DltSource, run_indexer_loop, run_sync_loop};
use node_storage::StorageBackend;
use tokio::sync::watch;

type SharedStorage = Arc<dyn StorageBackend>;

pub struct DltSyncWorker<Src> {
    store: SharedStorage,
    source: Src,
}

impl<Src> DltSyncWorker<Src>
where
    Src: DltSource,
{
    pub fn new(store: SharedStorage, source: Src) -> Self {
        Self { store, source }
    }

    pub fn sync_cursor(&self) -> watch::Receiver<Option<DltCursor>> {
        self.source.sync_cursor()
    }

    pub async fn run(self) -> anyhow::Result<()> {
        run_sync_loop(self.store.as_ref(), self.source).await // block forever
    }
}

pub struct DltIndexWorker {
    store: SharedStorage,
    index_interval: Duration,
}

impl DltIndexWorker {
    pub fn new(store: SharedStorage, index_interval: Duration) -> Self {
        Self { store, index_interval }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        loop {
            let result = run_indexer_loop(self.store.as_ref()).await;
            if let Err(e) = result {
                tracing::error!("{:?}", e);
            }
            tokio::time::sleep(self.index_interval).await;
        }
    }
}
