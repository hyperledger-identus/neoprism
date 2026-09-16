//! NeoPRISM compatibility facade over the generic `identus-crypto` crate.
//!
//! The beta branch keeps the historical `identus-apollo` package and module
//! paths so the rest of NeoPRISM can migrate independently. Cryptographic
//! mechanics are implemented by sdk-rust; this crate retains only narrow wire
//! and feature compatibility.

pub mod crypto;

#[cfg(feature = "hash")]
pub mod hash;

#[cfg(feature = "hex")]
pub mod hex;

#[cfg(feature = "base64")]
pub mod base64;

#[cfg(feature = "jwk")]
pub mod jwk;
