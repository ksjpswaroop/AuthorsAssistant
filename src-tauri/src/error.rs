use serde::{Serialize, Serializer};

/// Every command returns this on failure. It serialises to a plain string so the
/// frontend can show it without unwrapping a tagged enum.
#[derive(Debug)]
pub struct AppError(pub String);

pub type AppResult<T> = Result<T, AppError>;

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError(format!("file error: {e}"))
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError(format!("database error: {e}"))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError(format!("data error: {e}"))
    }
}

pub fn err<T>(msg: impl Into<String>) -> AppResult<T> {
    Err(AppError(msg.into()))
}
