#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("{message}")]
pub struct InvalidDid {
    message: String,
}

impl InvalidDid {
    pub(crate) fn new(message: &'static str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl From<identus_did::Error> for InvalidDid {
    fn from(source: identus_did::Error) -> Self {
        Self {
            message: source.to_string(),
        }
    }
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
#[display("invalid uri: {msg}")]
pub struct InvalidUri {
    pub msg: &'static str,
}

#[derive(Debug, derive_more::From, derive_more::Display, derive_more::Error)]
pub enum Error {
    #[display("{error}")]
    InvalidDid { error: InvalidDid },
    #[display("{error}")]
    InvalidUri { error: InvalidUri },
}
