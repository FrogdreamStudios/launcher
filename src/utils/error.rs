//! Error handling.

use std::fmt;

#[derive(Debug)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::new(format!("IO error: {err}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::new(format!("JSON error: {err}"))
    }
}

impl From<crate::backend::utils::net::http::Error> for Error {
    fn from(err: crate::backend::utils::net::http::Error) -> Self {
        Error::new(format!("HTTP error: {err}"))
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Error::new(format!("Task error: {err}"))
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Error::new(format!("Parse error: {err}"))
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::new(msg)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::new(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[macro_export]
macro_rules! simple_error {
    ($msg:literal) => {
        $crate::utils::error::Error::new($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::utils::error::Error::new(format!($fmt, $($arg)*))
    };
}
