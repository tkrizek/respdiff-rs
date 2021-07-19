extern crate lmdb;

use std::error::Error;
use std::path::Path;
use std::str;
use std::time::SystemTime;
use std::convert::TryInto;

use byteorder::{ByteOrder, LittleEndian};

use lmdb::Environment;
use lmdb::Transaction;
use lmdb::Cursor;
use lmdb::WriteFlags;
use lmdb::DatabaseFlags;

fn main() -> Result<(), Box<dyn Error>> {
    let env = Environment::new()
        .set_max_dbs(5)
        .set_map_size(10 * 1024_usize.pow(3))     // 10 G
        .set_max_readers(384)               // TODO: may need increasing?
        .open(Path::new("/tmp/respdiff-rs.db"))?;

    let db = env.open_db(Some("queries"))?;
    {
        let txn = env.begin_ro_txn()?;

        let mut cur = txn.open_ro_cursor(db)?;

        cur.iter().for_each(|res| {
            if let Ok((key, val)) = res {
                let key = LittleEndian::read_u32(&key);
                //println!("{:?} -> {:?}", key, val);
            }
        });
    }

    let ascii: Vec<u8> = "meta".into();

    let metadb = env.create_db(Some("meta"), DatabaseFlags::empty())?;
    {
        let ts: u32 = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs().try_into().unwrap();
        let mut ts_buf = [0; 4];
        LittleEndian::write_u32(&mut ts_buf, ts);

        let mut txn = env.begin_rw_txn()?;
        txn.put(metadb, b"version", b"2018-05-22", WriteFlags::empty())?;
        txn.put(metadb, b"start_time", &ts_buf, WriteFlags::empty())?;
        txn.commit()?;
    }
    {
        let txn = env.begin_ro_txn()?;
        //let vers: String = txn.get(metadb, b"version")?.into();
        let vers = txn.get(metadb, b"version")?;
        let ts = txn.get(metadb, b"start_time")?;
        println!("{:?} {:?}", vers, ts);
        let vers = str::from_utf8(vers)?;
        let ts = LittleEndian::read_u32(&ts);
        println!("{} {}", vers, ts);
    }

    Ok(())
}
