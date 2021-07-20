use lmdb::{DatabaseFlags, Database, Environment, Error as LmdbError};
use std::path::Path;

// Version string of supported respdiff db.
const BIN_FORMAT_VERSION: &str = "2018-05-21";

// Create an LMDB Environment. Only a single instance can exist in a process.
pub fn open_env(dir: &Path) -> Result<Environment, LmdbError> {
    Environment::new()
        .set_max_dbs(5)
        .set_map_size(10 * 1024_usize.pow(3))     // 10 G
        .set_max_readers(384)               // TODO: may need increasing?
        .open(dir)
}

// Create or open an LMDB database.
pub fn open_db(env: &Environment, name: &str, create: bool) -> Result<Database, LmdbError> {
    if create {
        env.create_db(Some(name), DatabaseFlags::empty())
    } else {
        env.open_db(Some(name))
    }
}

// Functions to work with the "meta" database.
pub mod metadb {
    use byteorder::{ByteOrder, LittleEndian};
    use lmdb::{Database, Error as LmdbError, RoTransaction, RwTransaction, Transaction, WriteFlags};
    use std::convert::TryInto;
    use std::error::Error;
    use std::time::SystemTime;

    pub const NAME: &str = "meta";

    pub fn write_version(db: Database, txn: &mut RwTransaction) -> Result<(), LmdbError> {
        txn.put(db, b"version", &super::BIN_FORMAT_VERSION, WriteFlags::empty())?;
        Ok(())
    }

    pub fn write_start_time(db: Database, txn: &mut RwTransaction) -> Result<(), Box<dyn Error>> {
        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        let ts: u32 = duration.as_secs().try_into()?;
        let mut bytes = [0; 4];
        LittleEndian::write_u32(&mut bytes, ts);
        txn.put(db, b"start_time", &bytes, WriteFlags::empty())?;
        Ok(())
    }

    pub fn read_version(db: Database, txn: &RoTransaction) -> Result<String, Box<dyn Error>> {
        let version = txn.get(db, b"version")?;
        Ok(String::from_utf8(version.to_vec())?)
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use lmdb::Transaction;
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
        let version = metadb::read_version(db, &txn).unwrap();

        assert_eq!(version, BIN_FORMAT_VERSION);
    }
}
