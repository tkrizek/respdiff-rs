use lmdb;
use std::error::Error;
use std::fmt;
use std::string::FromUtf8Error;
use std::result;

pub mod database;

#[derive(Debug)]
pub enum RespdiffError {
    Database(lmdb::Error),
    UnsupportedVersion,
    Time,
    NonAscii(FromUtf8Error),
}

impl fmt::Display for RespdiffError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            RespdiffError::Database(e) => write!(fmt, "database error: {}", e),
            RespdiffError::UnsupportedVersion => write!(fmt, "unsupported LMDB binary format"),
            RespdiffError::Time => write!(fmt, "failed to obtain current time"),
            RespdiffError::NonAscii(e) => write!(fmt, "non-ascii characters in conversion: {}", e),
        }
    }
}

impl Error for RespdiffError {}

impl From<lmdb::Error> for RespdiffError {
    fn from(error: lmdb::Error) -> Self {
        RespdiffError::Database(error)
    }
}

impl From<FromUtf8Error> for RespdiffError {
    fn from(error: FromUtf8Error) -> Self {
        RespdiffError::NonAscii(error)
    }
}

pub type Result<T> = result::Result<T, RespdiffError>;
