use lmdb;
use std::error::Error;
use std::fmt;
use std::time::SystemTimeError;

pub mod database;

#[derive(Debug, Eq, PartialEq)]
pub enum RespdiffError {
    Database(lmdb::Error),
    UnsupportedVersion,
    //Time(SystemTimeError),
}

impl fmt::Display for RespdiffError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            RespdiffError::Database(e) => write!(fmt, "database error: {}", e),
            RespdiffError::UnsupportedVersion => write!(fmt, "unsupported LMDB binary format"),
        }
    }
}

impl Error for RespdiffError {}

impl From<lmdb::Error> for RespdiffError {
    fn from(error: lmdb::Error) -> Self {
        RespdiffError::Database(error)
    }
}
