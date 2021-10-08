extern crate lmdb;

use async_std::{prelude::*, task};
use byteorder::{ByteOrder, LittleEndian};
use clap::{App, Arg};
use futures::channel::mpsc;
use lmdb::{Transaction, WriteFlags};
use log::error;
use respdiff::{
    config::Config,
    database::{self, queriesdb},
    error::Error,
    transceive,
};

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Duration;

struct Args {
    config: Config,
    envdir: PathBuf,
}

fn parse_args() -> Result<Args, Error> {
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
            let file = File::open(path).map_err(Error::ConfigFile)?;
            let buf = BufReader::new(file);
            serde_ini::from_bufread::<_, Config>(buf).map_err(Error::ConfigRead)?
        },
        envdir: { matches.value_of("ENVDIR").unwrap().into() },
    })
}

async fn transceiver() -> Result<(), Error> {
    let args = parse_args()?;

    let env = match database::open_env(&args.envdir) {
        Ok(env) => env,
        Err(e) => {
            error!("failed to open LMDB environment: {:?}", e);
            std::process::exit(1);
        }
    };
    let metadb = database::open_db(&env, database::metadb::NAME, true)?;
    {
        let mut txn = env.begin_rw_txn()?;
        database::metadb::write_servers(metadb, &mut txn, args.config.servers.clone())?;
        database::metadb::write_version(metadb, &mut txn)?;
        database::metadb::write_start_time(metadb, &mut txn)?;
        txn.commit()?;
    }

    // TODO consider this func
    //if database::exists_db(&env, &database::answersdb::NAME)? {
    //    error!("answers database already exists");
    //    std::process::exit(1);
    //}

    let qdb = database::open_db(&env, database::queriesdb::NAME, false)?;
    let txn = env.begin_ro_txn()?;
    let queries = queriesdb::get_queries(qdb, &txn)?;


    let servers = args.config.servers.iter().map(|name| args.config.server_data.get(name).unwrap().clone()).collect();
    let (rsender, mut rreceiver) = mpsc::unbounded();
    task::spawn(transceive::send_loop(queries, servers, rsender, Duration::from_secs_f64(args.config.sendrecv.timeout), 1500));  // TODO qps

    let adb = database::open_db(&env, database::answersdb::NAME, true)?;
    let mut txn = env.begin_rw_txn()?;
    task::block_on(async move {
        while let Some(responselist) = rreceiver.next().await {
            let key = responselist.key;
            let mut key_buf = [0; 4];
            LittleEndian::write_u32(&mut key_buf, key);
            let data: Vec<u8> = responselist.into();
            txn.put(adb, &key_buf, &data, WriteFlags::empty()).unwrap();  // TODO error handling?
        }
    });

    Ok(())
}

fn main() {
    env_logger::init();

    let res = task::block_on(transceiver());

    match res {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };
}
