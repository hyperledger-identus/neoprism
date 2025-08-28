# identus-did-core

This crate provides canonical Rust definitions for DID Documents and related types.

## TypeScript Type Export

To generate TypeScript type definitions for the canonical DID Document types, enable the `ts-types` feature and run tests:

```sh
cargo test --features ts-types
```

This will generate TypeScript files in the `bindings/` directory for all exported types.

### How it works
- The `ts-types` feature enables the optional `ts-rs` dependency and derives the necessary macros for TypeScript export.
- The test module at the end of `src/did_doc.rs` triggers export for all canonical types.
- Downstream crates do not pull in `ts-rs` unless they opt-in to the feature.

### Example
See `src/did_doc.rs` for annotated type definitions and the export test.

---

For more details, see the [ts-rs documentation](https://docs.rs/ts-rs/latest/ts_rs/).
