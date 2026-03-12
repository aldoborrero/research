use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlaveError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: status={status}, code={code}, message={message}")]
    Api {
        status: String,
        code: String,
        message: String,
    },

    #[error("Session not found or expired. Run `llave activate` first.")]
    NoSession,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("DNI/NIE authentication failed — the server did not redirect, which usually means the credentials (NIF, fecha, or soporte) are incorrect")]
    DniAuthFailed,

    #[error("Server returned HTML instead of JSON at {endpoint} — session may have expired or DNI auth may have failed")]
    HtmlResponse { endpoint: String },

    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Invalid NIF format: {0}")]
    InvalidNif(String),

    #[error("IP rate-limited by AEAT: {message}")]
    IpRateLimited { message: String },
}

pub type Result<T> = std::result::Result<T, LlaveError>;
