use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    HttpClient(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid HTTP method: {0}")]
    InvalidMethod(String),

    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),
}
