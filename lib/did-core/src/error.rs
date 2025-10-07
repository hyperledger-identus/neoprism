#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
#[display("{source}")]
pub struct InvalidDid {
    source: identity_did::Error,
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("invalid uri: {msg}")]
pub struct InvalidUri {
    pub msg: &'static str,
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[display("{_0}")]
    InvalidDid(InvalidDid),
    #[display("{_0}")]
    InvalidUri(InvalidUri),
}
