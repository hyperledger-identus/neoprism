# Implementation Plan: Blockfrost DLT Source

## Overview
Implement actual Blockfrost API integration in `blockfrost.rs` using the `blockfrost` crate to fetch PRISM metadata (label 21325) directly from Blockfrost's REST API. The implementation queries only blocks containing PRISM metadata for efficiency, uses pagination with size 100, retrieves transaction index (critical for ordering), and persists cursor after every block following the DbSync pattern.

## Requirements

**Functional Requirements:**
- Integrate with Blockfrost API using official `blockfrost` crate
- Fetch PRISM metadata transactions with label 21325 only
- Poll for new PRISM data at configurable intervals
- Filter blocks by confirmation depth (exclude recent unconfirmed blocks)
- Parse PRISM metadata from JSON format into `PublishedPrismObject`
- Persist cursor after every block processed
- Handle API errors and rate limits gracefully
- Restart stream worker on failures

**Non-Functional Requirements:**
- Must follow existing DbSync implementation pattern for consistency
- Must use blockfrost crate version 1.2 or later
- Must use pagination size of 100 for API calls
- Transaction index (absn) must be included in output (critical for ordering)
- Must handle API rate limits with retry logic
- Must use structured logging throughout
- Must not introduce panics on API failures

## Technical Decisions

**Architecture:**
- Async polling loop using tokio (matches DbSync pattern)
- Direct metadata query approach (query by label 21325, not iterate all blocks)
- Page-based pagination to control memory usage
- Worker spawn with restart loop for resilience

**Technology Stack:**
- `blockfrost = "1.2"` - Official Blockfrost Rust SDK (provides type-safe API, built-in retry, pagination support)
- Existing workspace dependencies (tokio, serde, tracing, identus-did-prism, identus-apollo)
- Pagination size 100 - balances API efficiency and memory usage (Blockfrost max per page)

**Design Patterns:**
- Polling pattern with configurable interval (same as DbSync)
- Cursor persistence via watch channel (same as DbSync)
- Helper functions for modular API interactions
- Error propagation using DltError type

**Integration Points:**
- Blockfrost API: `/metadata/txs/labels/21325` endpoint with pagination
- Blockfrost API: `/txs/{hash}` endpoint for transaction details
- Blockfrost API: `/blocks/latest` endpoint for cursor updates
- DltCursorRepo for cursor persistence (existing)
- event_tx channel for publishing `PublishedPrismObject` (existing)

## Context & Assumptions

**Constraints:**
- Must use blockfrost crate (as specified in tmp/prompt.md)
- Must follow existing code style and patterns in did-prism-indexer crate
- Must maintain compatibility with existing DltSource trait
- Cannot modify other DLT sources (oura, dbsync)
- Must work with existing CLI configuration (api_key, base_url, poll_interval)

**Assumptions:**
- Blockfrost API key will be provided via environment variable (NPRISM_BLOCKFROST_API_KEY)
- Blockfrost API returns transactions in chronological order within pagination
- Transaction metadata JSON format matches MetadataMapJson structure: `{"c": ["0x...", ...], "v": number}`
- Blockfrost API rate limits are manageable with default retry settings
- Confirmation blocks value (default 112) is appropriate for use case

**Risks:**
- API rate limits may cause delays if many PRISM transactions exist
- Pagination may miss transactions if new ones appear during pagination cycle
- Blockfrost API downtime would stop indexing (restart loop mitigates this)
- Metadata JSON format may vary from expected structure (error handling catches this)

**Scope Boundaries:**
- Included: Fetching PRISM metadata from Blockfrost API, parsing to PublishedPrismObject, cursor persistence, error handling, logging
- Out of scope: Modifying other DLT sources, changing DltSource trait, modifying CLI configuration, implementing CBOR metadata support (JSON only)

---

## Milestone 1: Add Dependencies and Update Cargo.toml

**Goal:** Add blockfrost dependency and enable blockfrost feature

### Implementation Steps

1. Open `/Cargo.toml` workspace dependencies section
2. Add `blockfrost = { version = "1.2", default-features = false }` to `[workspace.dependencies]` section
3. Open `lib/did-prism-indexer/Cargo.toml`
4. Locate the `blockfrost = [ ]` feature definition under `[features]` section (line 28)
5. Replace empty feature with: `blockfrost = ["dep:blockfrost"]`

### Verification
Run `cargo build --all-features` and verify no compilation errors related to missing blockfrost crate

---

## Milestone 2: Implement Data Structures and Parsing Logic

**Goal:** Define Blockfrost data structures and implement metadata parsing

### Implementation Steps

1. Open `lib/did-prism-indexer/src/dlt/blockfrost.rs`
2. Locate the `models` mod section (lines 18-46)
3. Replace `BlockfrostBlock` placeholder struct with:
   ```rust
   #[derive(Debug, Clone)]
   pub struct BlockfrostBlock {
       pub hash: String,
       pub height: u64,
       pub slot: u64,
       pub time: i64,
   }
   ```
4. Replace `BlockfrostMetadata` placeholder struct with:
   ```rust
   #[derive(Debug, Clone)]
   pub struct BlockfrostMetadata {
       pub tx_hash: String,
       pub tx_index: u32,
       pub json_metadata: serde_json::Value,
   }
   ```
5. Replace `parse_blockfrost_metadata` `todo!()` function (line 40-45) with implementation:
   - Parse `tx_hash` using `HexStr::from_str(&metadata.tx_hash)` to bytes, then `TxId::from_bytes()`
   - Handle parse errors: return `MetadataReadError::InvalidMetadataType` with block_hash and tx_index
   - Create `BlockMetadata` struct with: slot_number (from block.slot), block_number (from block.height), cbt (DateTime::from_timestamp(block.time, 0)), absn (from metadata.tx_index), tx_id (parsed)
   - Parse `json_metadata` to `MetadataMapJson` using `serde_json::from_value()`
   - Iterate through `metadata_json.c` array, for each string:
     - Check for "0x" prefix and split at 2 characters
     - Decode hex suffix using `HexStr::from_str(hex_suffix).to_bytes()`
     - Collect bytes into Vec<Vec<u8>>
   - Concatenate all byte arrays into single buffer
   - Decode protobuf: `PrismObject::decode(&bytes)` using `.map_err()` to convert to `MetadataReadError::PrismBlockProtoDecode`
   - Return `Ok(PublishedPrismObject { block_metadata, prism_object })`
6. Add error handling for each step with context (block_hash, tx_index)

### Verification
Run `cargo build --features blockfrost` and verify compilation succeeds with no type errors in models module

---

## Milestone 3: Implement Helper Functions

**Goal:** Create modular helper functions for Blockfrost API interactions

### Implementation Steps

1. Add imports to blockfrost.rs file:
   - `use blockfrost::{BlockfrostAPI, BlockFrostSettings, Order, Pagination};`
   - `use blockfrost_openapi::models::{block_content::BlockContent, tx_content::TxContent, tx_metadata_label_json_inner::TxMetadataLabelJsonInner};`
   - `use identus_did_prism::proto::MessageExt;`
2. Create `fetch_latest_confirmed_block` function (before `impl BlockfrostStreamWorker`):
   - Signature: `async fn fetch_latest_confirmed_block(api: &BlockfrostAPI, confirmation_blocks: u16) -> Result<BlockContent, DltError>`
   - Call `api.blocks_latest().await` and map error to `DltError::Connection`
   - Extract `tip_height` from `block.height` (cast to i64)
   - Calculate `confirmed_height = tip_height - confirmation_blocks as i64`
   - If confirmed_height < 0, return `DltError::Connection` (no confirmed blocks yet)
   - Fetch confirmed block using `api.blocks_by_id(&confirmed_height.to_string()).await` or return tip if confirmed_height == tip_height
   - Return block content
3. Create `fetch_prism_metadata_pages` function:
   - Signature: `async fn fetch_prism_metadata_pages(api: &BlockfrostAPI) -> Result<Vec<TxMetadataLabelJsonInner>, DltError>`
   - Initialize empty results vec
   - Set `page = 1`
   - Loop:
     - Create `Pagination::new(Order::Ascending, page, 100)`
     - Call `api.metadata_txs_by_label("21325", pagination).await`
     - Map errors to `DltError::Connection`
     - If result is empty (len == 0), break loop
     - Extend results vec with new items
     - Increment page
   - Return results vec
4. Create `get_block_for_tx` function:
   - Signature: `async fn get_block_for_tx(api: &BlockfrostAPI, tx_hash: &str) -> Result<(BlockfrostBlock, u32), DltError>`
   - Call `api.transaction_by_hash(tx_hash).await` and map error to `DltError::Connection`
   - Extract from `TxContent`: block (string), block_height (i64), block_time (i64), slot (i64), index (i32)
   - Parse block hash to bytes using `HexStr::from_str(&block_hash).to_bytes()` and convert to String
   - Return tuple: `BlockfrostBlock { hash: block_hash_string, height: block_height as u64, slot: slot as u64, time: block_time }, tx_index: index as u32`

### Verification
Run `cargo build --features blockfrost` and verify all three helper functions compile without errors

---

## Milestone 4: Implement BlockfrostAPI Client Setup

**Goal:** Create and configure Blockfrost client in stream worker

### Implementation Steps

1. Locate `BlockfrostStreamWorker::spawn` method (line 135)
2. After line 139 (where `base_url` and `api_key` are extracted), add:
   ```rust
   let settings = BlockFrostSettings {
       api: base_url.clone(),
       ..Default::default()
   };
   let api = Arc::new(BlockfrostAPI::new(&api_key, settings));
   ```
3. Update `stream_loop` call (lines 146-155) to pass API instance instead of individual parameters:
   - Remove parameters: `&api_key`, `&base_url`
   - Add parameter: `api: Arc<BlockfrostAPI>`
   - Update call to: `Self::stream_loop(api, event_tx.clone(), sync_cursor_tx.clone(), self.from_slot, self.confirmation_blocks, self.poll_interval).await`
4. Update `stream_loop` signature (line 170):
   - Remove: `_api_key: &str`, `_base_url: &str`
   - Add: `api: Arc<BlockfrostAPI>`
5. Add `use std::sync::Arc;` to imports if not present
6. Verify `api` variable is passed through correctly without ownership issues

### Verification
Run `cargo build --features blockfrost` and verify BlockfrostAPI client is created and passed to stream_loop without ownership or lifetime errors

---

## Milestone 5: Implement Cursor Persistence Method

**Goal:** Create cursor persistence utility matching DbSync pattern

### Implementation Steps

1. Add `use identus_apollo::hex::HexStr;` to imports (if not already present)
2. Create `persist_cursor` static method inside `impl BlockfrostStreamWorker` block (before or after `spawn` method):
   - Signature: `fn persist_cursor(block: &BlockfrostBlock, sync_cursor_tx: &watch::Sender<Option<DltCursor>>)`
   - Parse block hash to bytes: `let block_hash_bytes = HexStr::from_str(&block.hash).to_bytes();`
   - Create timestamp: `let cbt = DateTime::from_timestamp(block.time, 0).expect("valid timestamp");`
   - Create cursor: `let cursor = DltCursor { slot: block.slot, block_hash: block_hash_bytes, cbt: Some(cbt) };`
   - Send cursor: `let _ = sync_cursor_tx.send(Some(cursor));` (ignore send errors if channel closed)
   - Add logging: `tracing::debug!("Cursor persisted to slot={}, height={}", block.slot, block.height);`
3. Match DbSync implementation exactly (reference `lib/did-prism-indexer/src/dlt/dbsync.rs` line 306-316)

### Verification
Run `cargo build --features blockfrost` and verify persist_cursor method compiles and matches DbSync pattern

---

## Milestone 6: Implement `stream_loop` Polling Logic

**Goal:** Implement main polling loop that queries PRISM metadata, filters results, and persists cursor

### Implementation Steps

1. Replace `todo!()` in `stream_loop` method (line 179) with implementation
2. Initialize cursor tracking:
   - Get initial cursor from `sync_cursor_tx.borrow().as_ref().map(|c| c.slot).unwrap_or(from_slot)`
   - Set `current_slot = initial_cursor`
3. Start infinite `loop`:
   - Fetch latest confirmed block: `let confirmed_block = Self::fetch_latest_confirmed_block(&api, confirmation_blocks).await?;`
   - Fetch all PRISM metadata: `let prism_txs = Self::fetch_prism_metadata_pages(&api).await?;`
   - Filter and process transactions:
     - Initialize `new_prism_blocks = false`
     - For each `tx_meta` in `prism_txs`:
       - Get block data: `let (block, tx_index) = Self::get_block_for_tx(&api, &tx_meta.tx_hash).await?;`
       - Check filters: if `block.slot > current_slot` and `block.height <= confirmed_block.height`:
         - Create metadata: `BlockfrostMetadata { tx_hash: tx_meta.tx_hash.clone(), tx_index, json_metadata: tx_meta.json_metadata.clone() }`
         - Parse: `let prism_object = models::parse_blockfrost_metadata(block, metadata)?;`
         - Log detection: `tracing::info!("Detected PRISM metadata in tx={}, slot={}, index={}", tx_meta.tx_hash, block.slot, tx_index);`
         - Send to channel: `event_tx.send(prism_object).await.map_err(|e| DltError::EventHandling { source: e.to_string().into(), location: location!() })?;`
         - Persist cursor immediately: `Self::persist_cursor(&block, &sync_cursor_tx);`
         - Update state: `current_slot = block.slot; new_prism_blocks = true;`
   - Handle idle case:
     - If `!new_prism_blocks`:
       - Persist confirmed block cursor: `let block_hash = HexStr::from_str(&confirmed_block.hash.to_hex()).to_string(); let block_for_cursor = BlockfrostBlock { hash: block_hash, height: confirmed_block.height, slot: confirmed_block.slot, time: confirmed_block.time }; Self::persist_cursor(&block_for_cursor, &sync_cursor_tx);`
       - Sleep: `tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval)).await;`

### Verification
Run `cargo build --features blockfrost` and verify stream_loop compiles with all API calls, filtering logic, and cursor persistence

---

## Milestone 7: Add Error Handling and Logging

**Goal:** Add comprehensive error mapping and structured logging throughout

### Implementation Steps

1. Add error conversion at end of file (after `impl BlockfrostStreamWorker` block):
   ```rust
   impl From<blockfrost::BlockfrostError> for DltError {
       fn from(e: blockfrost::BlockfrostError) -> Self {
           tracing::error!("Blockfrost API error: {}", e);
           DltError::Connection { location: location!() }
       }
   }
   ```
2. Add logging imports (verify `use identus_did_prism::location;` is present)
3. Add logging in `stream_loop`:
   - After fetching latest block: `tracing::debug!("Fetched latest confirmed block: slot={}, height={}", confirmed_block.slot, confirmed_block.height);`
   - After fetching metadata: `tracing::debug!("Fetched {} PRISM metadata transactions", prism_txs.len());`
   - Wrap parse errors in `stream_loop`: if `parse_blockfrost_metadata` returns error, log warning instead of failing loop
4. Add logging in `fetch_latest_confirmed_block`:
   - Log successful fetch: `tracing::debug!("Latest block fetched: height={}, tip_height={}", confirmed_height, tip_height);`
   - Log when no confirmed blocks: `tracing::warn!("No confirmed blocks yet (confirmation_blocks={}, tip_height={})", confirmation_blocks, tip_height);`
5. Add logging in `fetch_prism_metadata_pages`:
   - Log after each page: `tracing::debug!("Fetched page {} with {} transactions", page, page_results.len());`
   - Log when no results: `tracing::debug!("No more PRISM metadata transactions found");`
6. Ensure all `?` operators propagate errors correctly to `DltError` type

### Verification
Run `cargo clippy --features blockfrost` and verify no warnings related to error handling or missing logging

---

## Milestone 8: Testing and Validation

**Goal:** Validate implementation with tests and integration checks

### Implementation Steps

1. Run unit tests: `just test --features blockfrost` or `cargo test --features blockfrost --package did-prism-indexer`
2. Run linter: `cargo clippy --features blockfrost -- -W clippy::all`
3. Format code: `cargo fmt` or `just format`
4. Verify feature compiles: `cargo build --features blockfrost --release`
5. If environment available, perform integration test:
   - Set `NPRISM_BLOCKFROST_API_KEY` environment variable with valid Blockfrost API key
   - Run node: `nix develop` then `cargo run --bin neoprism-node --features blockfrost -- --dlt-source blockfrost`
   - Verify startup logs show "Starting Blockfrost stream worker"
   - Verify logs show "Fetched latest confirmed block"
   - Wait for PRISM metadata detection and verify logs show transaction details (tx_hash, slot, index)
   - Verify logs show "Cursor persisted to slot=" messages
   - Verify no panic or error logs during normal operation

### Verification
- All tests pass: `just test --features blockfrost`
- No clippy warnings: `cargo clippy --features blockfrost`
- Code formatted: `cargo fmt` reports no changes
- Binary compiles: `cargo build --features blockfrost --release` succeeds
- Integration test (if available): Node starts successfully, fetches blocks, detects PRISM metadata, persists cursor without errors

---

