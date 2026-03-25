use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] ureq::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("JMAP error in {method}: {message}")]
    Jmap { method: String, message: String },

    #[error("invalid params: {0}")]
    InvalidParams(String),

    #[error("no API token found. Set FASTMAIL_API_TOKEN or run `fm setup`")]
    MissingToken,
}

pub type Result<T> = std::result::Result<T, Error>;
