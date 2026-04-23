use thiserror::Error;

pub type Result<T> = std::result::Result<T, WorldGenError>;

#[derive(Debug, Error)]
pub enum WorldGenError {
    #[error("{0}")]
    MissingOption(String),
    #[error("{0}")]
    InvalidOption(String),
    #[error("{0}")]
    InvalidData(String),
    #[error("{0}")]
    State(String),
    #[error("Chunk schema version {actual} is not supported by this build. Expected {expected}. Run the documented migration flow before loading this save.")]
    UnsupportedSchemaVersion { actual: i32, expected: i32 },
    #[error("Operation canceled.")]
    Cancelled,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Encode(#[from] rmp_serde::encode::Error),
    #[error(transparent)]
    Decode(#[from] rmp_serde::decode::Error),
}

impl WorldGenError {
    pub fn missing_option(name: &str) -> Self {
        Self::MissingOption(format!("Missing required option --{name}."))
    }

    pub fn invalid_option(message: impl Into<String>) -> Self {
        Self::InvalidOption(message.into())
    }

    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self::InvalidData(message.into())
    }

    pub fn state(message: impl Into<String>) -> Self {
        Self::State(message.into())
    }
}
