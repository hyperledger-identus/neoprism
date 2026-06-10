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
**Severity:** Low (cursor advances past a failed event, but stream terminates immediately after)
**Discovered:** 2026-06-11

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

### Expected Behavior

Check the result first, only advance cursor on success:

```rust
for row in metadata_rows {
    Self::process_prism_object(row.clone(), &event_tx).await?;
    Self::emit_cursor_progress(row.into(), &sync_cursor_tx);
}
```

### Actual Behavior

Cursor is always advanced, even when processing fails. The cursor update is discarded when the stream loop restarts (it uses the persisted cursor), so this is benign in practice.
