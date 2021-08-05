use std::result;

pub mod config;
pub mod database;

mod error;
pub use error::Error as Error;

pub type Result<T> = result::Result<T, Error>;
