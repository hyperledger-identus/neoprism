# Bugs Found During Coverage Improvement

## Bug 1: `X25519PublicKey::from_slice` silently accepts oversized input

**File:** `lib/apollo/src/crypto/x25519.rs`
**Function:** `X25519PublicKey::from_slice(slice: &[u8])`
**Severity:** Low (silent truncation of extra bytes)
**Discovered:** 2026-06-11

### Description

`from_slice` uses `split_first_chunk::<32>()` to extract the first 32 bytes from the input slice. If the input is longer than 32 bytes, the extra bytes are silently ignored rather than producing an error. Compare with `Ed25519PublicKey::from_slice` which uses `split_first_chunk` similarly but has a note about this behavior.

For a cryptographic key parsing function, accepting oversized input without error could mask bugs in callers that accidentally pass data with trailing bytes (e.g., a key with an appended tag or length prefix).

### Expected Behavior

The function should return `Err(Error::InvalidKeySize { ... })` when the input is not exactly 32 bytes, consistent with how it rejects inputs shorter than 32 bytes.

### Actual Behavior

```rust
let key = X25519PublicKey::from_slice(&[0u8; 33]);
assert!(key.is_ok()); // silently takes first 32 bytes
```

### Suggested Fix

Replace `split_first_chunk::<32>()` with an exact size check:

```rust
if slice.len() != 32 {
    return Err(Error::InvalidKeySize { ... });
}
let key = x25519_dalek::PublicKey::from(slice.try_into().unwrap());
```

Or keep `split_first_chunk` but add an additional check that the remainder is empty.

---

## Bug 2: `DbSyncStreamWorker::emit_cursor_progress` called before `process_prism_object` error check

**File:** `lib/did-prism-indexer/src/dlt/dbsync.rs`
**Function:** `DbSyncStreamWorker::stream_loop`
**Severity:** Low (benign in practice — not fixed)
**Discovered:** 2026-06-11
**Resolution:** Intentionally left as-is. See the comment in `stream_loop` in `dbsync.rs`.

### Description

In `stream_loop`, the cursor is advanced via `emit_cursor_progress` *before* checking the result of `process_prism_object`. If processing fails (channel closed), the cursor has already advanced past that row, but since the error causes the stream loop to terminate entirely, this is not a practical issue — the cursor update is lost when the loop restarts from the persisted cursor.

```rust
for row in metadata_rows {
    let process_result = Self::process_prism_object(row.clone(), &event_tx).await;
    Self::emit_cursor_progress(row.into(), &sync_cursor_tx);  // cursor advanced
    if let Err(e) = process_result {                          // then checked
        // ...
        return Err(e);                                        // loop terminates
    }
}
```

### Why Not Fixed

The persisted cursor (read from the DB on startup) is the source of truth — the in-memory `sync_cursor_tx` watch channel is just a side channel. On error, the loop returns `Err(e)` and the spawned task restarts (see `into_stream` / `DbSyncSource::stream_loop`); the in-memory cursor update is discarded. Advancing the cursor first is therefore equivalent in effect to advancing it after, and reordering would lose the explicit `tracing::error!` log lines on the error path. The current code is kept as-is and documented inline in `dbsync.rs` to prevent accidental "fixes."

### Expected (intended) Behavior

Process row → advance cursor → if processing failed, log and return. Cursor ordering is irrelevant in practice because the loop terminates on error and restarts from the persisted cursor.

---

## Bug 3: `SqliteDb::set_cursor` leaves stale rows in `dlt_cursor`

**File:** `lib/node-storage/src/backend/sqlite.rs`
**Function:** `SqliteDb::set_cursor`
**Severity:** High (indexer resumes from stale slot on every restart)
**Discovered:** 2026-06-11
**Resolution:** Fixed in the same PR. The cursor-deletion loop is replaced with a single raw `DELETE FROM dlt_cursor`, and the regression test (`set_cursor_replaces_previous_cursor`) now asserts correct behavior.

### Description

`set_cursor` was implemented as "list existing cursors, delete each by id, then insert the new one". The lazybe/sea_query delete emits a parameterised `id = ?` statement where the bound value is `uuid::Uuid` rendered as a hyphenated string, but the `dlt_cursor.id` column in SQLite is `BLOB` (16 raw bytes from `randomblob(16)`). The string-vs-blob comparison never matches, so the delete silently affects zero rows.

The table is logically a singleton (at most one cursor row at a time), so the loop could only ever leave N-1 stale rows behind after the Nth call to `set_cursor`. In practice this manifested as:

- `SELECT COUNT(*) FROM dlt_cursor` returning a value > 1 after a few `set_cursor` calls.
- `get_cursor` returning the *first* inserted row (since the list query returns all rows in insertion order and the first is taken). The latest cursor was effectively unreachable.

For the indexer, this means `get_cursor` on startup returns a slot from the first persisted run, and the indexer reprocesses (or skips) the same blocks on every restart.

### Why Raw `DELETE` Instead of Fixing the Lazybe Binding

The sea_query SqliteQueryBuilder encodes `uuid::Uuid` as a hyphenated string, but the column is `BLOB`. Two ways to fix:

1. Change the column to `TEXT` and store Uuids as strings. This requires a migration and changes the storage format.
2. Use raw SQL for the delete. The table holds at most one row in practice, so `DELETE FROM dlt_cursor` is unconditional and bypasses the type mismatch entirely.

Option 2 is smaller and contained to the SQLite backend. The Postgres backend is unaffected (the column is native `uuid` there and `list`/`delete` work as written).
