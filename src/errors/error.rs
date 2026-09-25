use thiserror::Error;

#[derive(Error, Debug)]
pub enum PowerError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("hardware interface error: {0}")]
    Hardware(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("permission denied")]
    PermissionDenied,

    #[error("operation blocked by inhibitor(s): {0:?}")]
    Inhibited(Vec<String>),

    #[error("invalid state: {0}")]
    InvalidState(String),

    #[error("unsupported on this hardware: {0}")]
    Unsupported(String),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("config parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("invalid parameters: {0}")]
    InvalidParams(String),

    #[error("unknown method: {0}")]
    UnknownMethod(String),

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, PowerError>;
