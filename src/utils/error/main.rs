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
        Self::new(format!("IO error: {err}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::new(format!("JSON error: {err}"))
    }
}



impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Self::new(format!("Task main: {err}"))
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Self::new(format!("Parse main: {err}"))
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Self::new(msg)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Self::new(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[macro_export]
macro_rules! simple_error {
    ($msg:literal) => {
        $crate::utils::error::main::Error::new($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::utils::error::main::Error::new(format!($fmt, $($arg)*))
    };
}
