//! Custom utilities.

pub mod archive;
pub mod error;
pub mod hex;
pub mod logger;
pub mod net;
pub mod sha1;
pub mod which;

pub use archive::main::extract_zip;
pub use error::main::{Error, Result};
pub use hex::main::encode as hex_encode;

pub use sha1::main::{Digest, Sha1};
pub use which::main::which;
