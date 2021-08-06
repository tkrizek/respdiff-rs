use lmdb;
use std::fmt;
use std::io;
use std::string::FromUtf8Error;
use serde_ini::de;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database error: {0}")]
    Database(#[from] lmdb::Error),
    #[error("unsupported LMDB binary format")]
    UnsupportedVersion,
    #[error("failed to obtain current time")]
    Time,
    #[error("non-ascii characters in conversion: {0}")]
    NonAscii(#[from] FromUtf8Error),
    #[error("unknown transport protocol: {0}")]
    UnknownTransportProtocol(String),
    #[error("unknown diff criteria: {0}")]
    UnknownDiffCriteria(String),
    #[error("unknown field weight: {0}")]
    UnknownFieldWeight(String),
    #[error("failed to open config file: {0}")]
    ConfigFile(io::Error),
    #[error("failed to parse config file: {0}")]
    ConfigRead(de::Error),
    #[error("functionality not yet implemented")]
    NotImplemented,
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
            (NotImplemented, NotImplemented) => true,
            _ => false,
        }
    }
}
impl Eq for Error {}
