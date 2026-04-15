#![cfg_attr(
    any(feature = "oura", feature = "dbsync", feature = "blockfrost"),
    feature(error_reporter)
)]

use identus_did_prism::dlt::{DltCursor, PublishedPrismObject};
use tokio::sync::{mpsc, watch};

pub mod dlt;
mod indexing;
pub mod repo;

pub use indexing::{run_indexer_loop, run_sync_loop};

pub trait DltSource {
    fn sync_cursor(&self) -> watch::Receiver<Option<DltCursor>>;
    fn into_stream(self) -> Result<mpsc::Receiver<PublishedPrismObject>, String>;
}
