use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClaveError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: status={status}, code={code}, message={message}")]
    Api {
        status: String,
        code: String,
        message: String,
    },

    #[error("Session not found or expired. Run `clave activate` first.")]
    NoSession,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Invalid NIF format: {0}")]
    InvalidNif(String),
}

pub type Result<T> = std::result::Result<T, ClaveError>;
