use byteorder::{ByteOrder, LittleEndian};
use lmdb::{Cursor, DatabaseFlags, Environment, Error as LmdbError, Transaction, WriteFlags};

use std::path::Path;

pub fn open_env(dir: &Path) -> Result<Environment, LmdbError> {
    Environment::new()
        .set_max_dbs(5)
        .set_map_size(10 * 1024_usize.pow(3))     // 10 G
        .set_max_readers(384)               // TODO: may need increasing?
        .open(dir)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
