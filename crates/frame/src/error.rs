//! Frame errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("insufficient authority: required {required}, found {found}")]
    InsufficientAuthority { required: String, found: String },

    #[error("unknown capability: {0}")]
    UnknownCapability(String),

    #[error("missing required parameter: {0}")]
    MissingParam(String),

    #[error("invalid node identity: {0}")]
    InvalidIdentity(String),

    #[error("serialization: {0}")]
    Serialization(#[from] serde_json::Error),
}
