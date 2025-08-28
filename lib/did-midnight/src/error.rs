#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[display("invalid did syntax: {input}")]
    InvalidDidSyntax { input: String },
    #[display("invalid method: {method}")]
    InvalidMethod { method: String },
    #[display("invalid segment count: {found}")]
    InvalidSegmentCount { found: usize },
    #[display("invalid network: {source}")]
    #[from]
    InvalidNetwork { source: strum::ParseError },
    #[display("invalid address length: found {found}, expected {expected}")]
    InvalidAddressLength { found: usize, expected: usize },
    #[display("invalid address hex: {source}")]
    #[from]
    InvalidAddressHex { source: identus_apollo::hex::Error },
    #[display("invalid address case")]
    InvalidAddressCase,
}
