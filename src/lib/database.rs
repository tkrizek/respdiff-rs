use lmdb::{DatabaseFlags, Database, Environment, Error as LmdbError};
use std::path::Path;

use crate::{Result, RespdiffError};

// Version string of supported respdiff db.
const BIN_FORMAT_VERSION: &str = "2018-05-21";

// Create an LMDB Environment. Only a single instance can exist in a process.
pub fn open_env(dir: &Path) -> Result<Environment> {
    Ok(Environment::new()
        .set_max_dbs(5)
        .set_map_size(10 * 1024_usize.pow(3))     // 10 G
        .set_max_readers(384)               // TODO: may need increasing?
        .open(dir)?)
}

// Create or open an LMDB database.
pub fn open_db(env: &Environment, name: &str, create: bool) -> Result<Database> {
    if create {
        Ok(env.create_db(Some(name), DatabaseFlags::empty())?)
    } else {
        Ok(env.open_db(Some(name))?)
    }
}

// Check if database exists already.
pub fn exists_db(env: &Environment, name: &str) -> Result<bool> {
    match env.open_db(Some(name)) {
        Ok(_) => Ok(true),
        Err(LmdbError::NotFound) => Ok(false),
        Err(e) => Err(RespdiffError::Database(e)),
    }
}

// Functions to work with the "meta" database.
pub mod metadb {
    use byteorder::{ByteOrder, LittleEndian};
    use lmdb::{Database, RoTransaction, RwTransaction, Transaction, WriteFlags};
    use std::convert::TryInto;
    use std::time::SystemTime;
    use crate::{Result, RespdiffError};

    pub const NAME: &str = "meta";

    pub fn write_version(db: Database, txn: &mut RwTransaction) -> Result<()> {
        Ok(txn.put(db, b"version", &super::BIN_FORMAT_VERSION, WriteFlags::empty())?)
    }

    pub fn write_start_time(db: Database, txn: &mut RwTransaction) -> Result<()> {
        let duration = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(val) => val,
            Err(_) => return Err(RespdiffError::Time),
        };
        let ts: u32 = match duration.as_secs().try_into() {
            Ok(val) => val,
            Err(_) => return Err(RespdiffError::Time),
        };
        let mut bytes = [0; 4];
        LittleEndian::write_u32(&mut bytes, ts);
        Ok(txn.put(db, b"start_time", &bytes, WriteFlags::empty())?)
    }

    pub fn check_version(db: Database, txn: &RoTransaction) -> Result<String> {
        let version = txn.get(db, b"version")?;
        let version = String::from_utf8(version.to_vec())?;

        if version == super::BIN_FORMAT_VERSION {
            Ok(version)
        } else {
            Err(RespdiffError::UnsupportedVersion)
        }
    }
}

pub mod queriesdb {
    use std::convert::TryFrom;
    use byteorder::{ByteOrder, LittleEndian};
    use log::warn;

    pub const NAME: &str = "queries";

    #[derive(Debug)]
    pub struct Query {
        pub key: u32,
        pub wire: Vec<u8>,
    }

    impl TryFrom<lmdb::Result<(&[u8], &[u8])>> for Query {
        type Error = lmdb::Error;

        fn try_from(item: lmdb::Result<(&[u8], &[u8])>) -> Result<Self, Self::Error> {
            match item {
                Ok((key, val)) => {
                    Ok(Query {
                        key: LittleEndian::read_u32(&key),
                        wire: val.to_vec(),
                    })
                },
                Err(e) => {
                    warn!("failed to read query from db");
                    Err(e)
                },
            }
        }
    }
}

pub mod answersdb {
    pub const NAME: &str = "answers";
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use lmdb::{Error as LmdbError, Transaction};
    use super::*;

    #[test]
    fn metadb_version() {
        let dir = TempDir::new("test").unwrap();
        let env = open_env(dir.path()).unwrap();
        let db = open_db(&env, metadb::NAME, true).unwrap();

        let mut txn = env.begin_rw_txn().unwrap();
        metadb::write_version(db, &mut txn).unwrap();
        txn.commit().unwrap();

        let txn = env.begin_ro_txn().unwrap();
        let version = metadb::check_version(db, &txn).unwrap();
        assert_eq!(version, BIN_FORMAT_VERSION);
    }

    #[test]
    fn exists() {
        let dir = TempDir::new("test").unwrap();
        let env = open_env(dir.path()).unwrap();
        let _d1 = open_db(&env, "d1", true).unwrap();

        assert_eq!(exists_db(&env, "d1").unwrap(), true);
        assert_eq!(exists_db(&env, "x").unwrap(), false);

        // trigger DbsFull becuase we set db limit to 5
        let _d2 = open_db(&env, "d2", true).unwrap();
        let _d3 = open_db(&env, "d3", true).unwrap();
        let _d4 = open_db(&env, "d4", true).unwrap();
        let _d5 = open_db(&env, "d5", true).unwrap();

        assert_eq!(exists_db(&env, "x"), Err(RespdiffError::Database(LmdbError::DbsFull)));
    }
}
