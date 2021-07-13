extern crate lmdb;

use std::error::Error;
use std::path::Path;
use lmdb::Environment;
use lmdb::Transaction;
use lmdb::Cursor;

fn main() -> Result<(), Box<dyn Error>> {
    let env = Environment::new()
        .set_max_dbs(5)
        .set_map_size(10 * 1024_usize.pow(3))     // 10 G
        .set_max_readers(384)               // TODO: may need increasing?
        .open(Path::new("/tmp/respdiff-rs.db"))?;

    let db = env.open_db(Some("queries"))?;
    let txn = env.begin_ro_txn()?;

    let mut cur = txn.open_ro_cursor(db)?;

    cur.iter().for_each(|res| {
        if let Ok((key, val)) = res {
            println!("{:?} -> {:?}", key, val);
        }
    });

    Ok(())
}
