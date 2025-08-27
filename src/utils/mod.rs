//! Custom utilities.

pub mod archive;

pub mod error;
pub mod hex;
pub mod logging;
pub mod sha1;
pub mod which;

pub use archive::{extract_tar_gz, extract_zip};
pub use error::{Error, Result};
pub use hex::encode as hex_encode;

pub use sha1::{Digest, Sha1};
pub use which::which;
