use std::{fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Corruption(String),
    TamperDetected(String),
    InvalidFormat(String),
    InvalidInput(String),
    DuplicateTransaction(String),
    Locked(String),
    Poisoned,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Corruption(s) => write!(f, "corruption: {s}"),
            Self::TamperDetected(s) => write!(f, "tamper detected: {s}"),
            Self::InvalidFormat(s) => write!(f, "invalid format: {s}"),
            Self::InvalidInput(s) => write!(f, "invalid input: {s}"),
            Self::DuplicateTransaction(s) => write!(f, "transaction ID already exists: {s}"),
            Self::Locked(s) => write!(f, "database lock unavailable: {s}"),
            Self::Poisoned => write!(
                f,
                "a write failed; close and reopen the database before continuing"
            ),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::InvalidFormat(value.to_string())
    }
}
