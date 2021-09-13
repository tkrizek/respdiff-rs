extern crate lmdb;

use clap::{App, Arg};
use lmdb::{Cursor, Transaction};
use log::error;
use respdiff::{
    self,
    config::Config,
    database::{self, queriesdb::Query},
};

use std::convert::TryFrom;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

struct Args {
    config: Config,
    envdir: PathBuf,
}

fn parse_args() -> Result<Args, respdiff::Error> {
    let matches = App::new("Respdiff: Transceiver")
        .about("send queries to servers and record answers (replaces orchestrator.py)")
        .arg(
            Arg::with_name("ENVDIR")
                .help("LMDB environment directory")
                .required(true),
        )
        .arg(
            Arg::with_name("config")
                .help("config file path")
                .short("c")
                .long("config")
                .value_name("FILE")
                .takes_value(true),
        )
        .get_matches();

    Ok(Args {
        config: {
            let path = matches.value_of("config").unwrap_or("respdiff.cfg");
            let file = File::open(path).map_err(respdiff::Error::ConfigFile)?;
            let buf = BufReader::new(file);
            serde_ini::from_bufread::<_, Config>(buf).map_err(respdiff::Error::ConfigRead)?
        },
        envdir: { matches.value_of("ENVDIR").unwrap().into() },
    })
}

fn transceiver() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;

    let env = match database::open_env(&args.envdir) {
        Ok(env) => env,
        Err(e) => {
            error!("failed to open LMDB environment: {:?}", e);
            std::process::exit(1);
        }
    };
    let metadb = match database::open_db(&env, database::metadb::NAME, true) {
        Ok(db) => db,
        Err(e) => {
            error!(
                "failed to open LMDB database '{}': {:?}",
                &database::metadb::NAME,
                e
            );
            std::process::exit(1);
        }
    };

    {
        let mut txn = env.begin_rw_txn()?;
        database::metadb::write_servers(metadb, &mut txn, args.config.servers)?;
        database::metadb::write_version(metadb, &mut txn)?;
        database::metadb::write_start_time(metadb, &mut txn)?;
        txn.commit()?;
    }

    //if database::exists_db(&env, &database::answersdb::NAME)? {
    //    error!("answers database already exists");
    //    std::process::exit(1);
    //}
    let _adb = database::open_db(&env, database::answersdb::NAME, true)?;

    let qdb = match database::open_db(&env, database::queriesdb::NAME, false) {
        Ok(db) => db,
        Err(e) => {
            error!(
                "failed to open LMDB database '{}': {:?}",
                &database::queriesdb::NAME,
                e
            );
            std::process::exit(1);
        }
    };

    {
        let txn = env.begin_ro_txn()?;
        let mut cur = txn.open_ro_cursor(qdb)?;
        let iter = cur.iter().map(Query::try_from);
        iter.for_each(|query| {
            println!("{:?}", query);
        });
    }

    Err(Box::new(respdiff::Error::NotImplemented))
}

fn main() {
    env_logger::init();

    match transceiver() {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };
}
