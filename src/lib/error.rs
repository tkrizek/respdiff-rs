use lmdb;
use std::fmt;
use std::io;
use std::string::FromUtf8Error;
use serde_ini::de;

#[derive(Debug)]
pub enum Error {
    Database(lmdb::Error),
    UnsupportedVersion,
    Time,
    NonAscii(FromUtf8Error),
    UnknownTransportProtocol(String),
    UnknownDiffCriteria(String),
    UnknownFieldWeight(String),
    ConfigFile(io::Error),
    ConfigRead(de::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Error::Database(e) => write!(fmt, "database error: {}", e),
            Error::UnsupportedVersion => write!(fmt, "unsupported LMDB binary format"),
            Error::Time => write!(fmt, "failed to obtain current time"),
            Error::NonAscii(e) => write!(fmt, "non-ascii characters in conversion: {}", e),
            Error::UnknownTransportProtocol(s) => write!(fmt, "unknown transport protocol: {}", s),
            Error::UnknownDiffCriteria(s) => write!(fmt, "unknown diff criteria: {}", s),
            Error::UnknownFieldWeight(s) => write!(fmt, "unknown field weight: {}", s),
            Error::ConfigFile(e) => write!(fmt, "config file error: {}", e),
            Error::ConfigRead(e) => write!(fmt, "failed to read config: {}", e),
        }
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        use Error::*;
        match (self, other) {
            (Database(a), Database(b)) => a == b,
            (UnsupportedVersion, UnsupportedVersion) => true,
            (Time, Time) => true,
            (NonAscii(a), NonAscii(b)) => a == b,
            (UnknownTransportProtocol(a), UnknownTransportProtocol(b)) => a == b,
            (UnknownDiffCriteria(a), UnknownDiffCriteria(b)) => a == b,
            (UnknownFieldWeight(a), UnknownFieldWeight(b)) => a == b,
            (ConfigFile(_), ConfigFile(_)) => true,
            (ConfigRead(_), ConfigRead(_)) => true,
            _ => false,
        }
    }
}
impl Eq for Error {}

impl std::error::Error for Error {}

impl From<lmdb::Error> for Error {
    fn from(error: lmdb::Error) -> Self {
        Error::Database(error)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Self {
        Error::NonAscii(error)
    }
}
