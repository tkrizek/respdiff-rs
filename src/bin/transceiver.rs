extern crate lmdb;

use std::error::Error;
use std::path::Path;
use lmdb::Transaction;
use log::error;
use respdiff::database;

fn transceiver() -> Result<(), Box<dyn Error>> {
    let dir = Path::new("/tmp/respdiff-rs.db");
    let env = match database::open_env(dir) {
        Ok(env) => env,
        Err(e) => {
            error!("failed to open LMDB environment: {:?}", e);
            std::process::exit(1);
        },
    };
    let metadb = match database::open_db(&env, &database::metadb::NAME, true) {
        Ok(db) => db,
        Err(e) => {
            error!("failed to open LMDB database '{}': {:?}", &database::metadb::NAME, e);
            std::process::exit(1);
        },
    };
    //let querydb = match database::open_db(&env, &database::querydb::NAME, true) {
    //    Ok(db) => db,
    //    Err(e) => panic!(
    //        "Failed to open LMDB database '{s}': {:?}",
    //        &database::querydb::NAME,
    //        e);
    //}

    //{
    //    let txn = db.begin_ro_txn()?;

    //    let mut cur = txn.open_ro_cursor(db)?;

    //    cur.iter().for_each(|res| {
    //        if let Ok((key, val)) = res {
    //            let key = LittleEndian::read_u32(&key);
    //            //println!("{:?} -> {:?}", key, val);
    //        }
    //    });
    //}

    {
        let mut txn = env.begin_rw_txn()?;
        database::metadb::write_version(metadb, &mut txn)?;
        database::metadb::write_start_time(metadb, &mut txn)?;
        txn.commit()?;
    }
    {
        let txn = env.begin_ro_txn()?;
        let version = database::metadb::check_version(metadb, &txn)?;
        println!("version: {}", version);
    }



    if database::exists_db(&env, &database::answersdb::NAME)? {
        error!("answers database already exists");
        std::process::exit(1);
    }

    Ok(())
}

fn main() {
    env_logger::init();

    match transceiver() {
        Ok(_) => {},
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        },
    };
}
