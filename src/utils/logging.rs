//! Minimal logging system.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum LogLevel {
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
}

impl LogLevel {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warn" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
        }
    }
}

static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);

pub fn init(level: LogLevel) {
    LOG_LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn init_from_env() {
    let level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| LogLevel::from_str(&s))
        .unwrap_or(if cfg!(debug_assertions) {
            LogLevel::Debug
        } else {
            LogLevel::Info
        });
    init(level);
}

pub fn enabled(level: LogLevel) -> bool {
    (level as u8) <= LOG_LEVEL.load(Ordering::Relaxed)
}

pub fn log(level: LogLevel, message: &str) {
    if enabled(level) {
        println!("[{}] {}", level.as_str(), message);
    }
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::utils::logging::log($crate::utils::logging::LogLevel::Error, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::utils::logging::log($crate::utils::logging::LogLevel::Warn, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::utils::logging::log($crate::utils::logging::LogLevel::Info, &format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::utils::logging::log($crate::utils::logging::LogLevel::Debug, &format!($($arg)*))
    };
}
