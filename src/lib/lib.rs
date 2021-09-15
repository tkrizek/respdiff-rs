use std::result;

/// Configuration file.
pub mod config;
/// Utilities for working with LMDB database.
pub mod database;
/// JSON data format.
pub mod dataformat;
/// Logic for comparing DNS messages.
pub mod matcher;

mod error;
pub use error::Error;

/// Respdiff result type.
pub type Result<T> = result::Result<T, Error>;
