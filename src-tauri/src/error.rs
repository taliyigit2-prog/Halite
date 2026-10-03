use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Audio decode error: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
    #[error("Audio encode error: {0}")]
    Hound(#[from] hound::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("ffmpeg error: {0}")]
    Ffmpeg(String),
    #[error("ONNX Runtime error: {0}")]
    Ort(String),
    #[error("{0}")]
    Message(String),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
