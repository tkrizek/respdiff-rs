use std::result;

pub mod config;
pub mod database;
pub mod matcher;

mod error;
pub use error::Error as Error;

pub type Result<T> = result::Result<T, Error>;
