extern crate lmdb;

use std::convert::TryFrom;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use clap::{Arg, App};
use lmdb::{Cursor, Transaction};
use log::error;
use respdiff::{self, config::Config, database::{self, answersdb::ServerReplyList}};
use serde_ini;

struct Args {
    config: Config,
    //datafile: PathBuf,
    envdir: PathBuf,
}

fn parse_args() -> Result<Args, respdiff::Error> {
    let matches = App::new("Respdiff: Msgdiff")
        .about("find differences between answers")
        .arg(Arg::with_name("ENVDIR")
            .help("LMDB environment directory")
            .required(true))
        .arg(Arg::with_name("config")
            .help("config file path")
            .short("c")
            .long("config")
            .value_name("FILE")
            .takes_value(true))
//        TODO generarate JSON
//        .arg(Arg::with_name("datafile")
//            .help("JSON report file")
//            .short("d")
//            .long("datafile")
//            .value_name("FILE")
//            .takes_value(true))
        .get_matches();

    let envdir: PathBuf = matches.value_of("ENVDIR").unwrap().into();

    Ok(Args {
        config: {
            let path = matches.value_of("config").unwrap_or("respdiff.cfg");
            let file = File::open(path).map_err(respdiff::Error::ConfigFile)?;
            let buf = BufReader::new(file);
            serde_ini::from_bufread::<_, Config>(buf).map_err(respdiff::Error::ConfigRead)?
        },
//        datafile: {
//            let datafile = matches.value_of("datafile").unwrap_or(envdir + "respdiff.cfg"
//        },
        envdir: envdir,
    })
}

fn msgdiff() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;

    let env = match database::open_env(&args.envdir) {
        Ok(env) => env,
        Err(e) => {
            error!("failed to open LMDB environment: {:?}", e);
            std::process::exit(1);
        },
    };

    let adb = database::open_db(&env, &database::answersdb::NAME, false)?;
    {
        let txn = env.begin_ro_txn()?;
        let mut cur = txn.open_ro_cursor(adb)?;

        let iter = cur.iter().map(|res| {
            match res {
                Ok(item) => Ok(ServerReplyList::try_from(item)),
                Err(e) => Err(respdiff::Error::Database(e)),
            }
        });
        for i in iter {
            println!("key: {}", i??.key);
        }
    }

    Err(Box::new(respdiff::Error::NotImplemented))
}

fn main() {
    env_logger::init();

    match msgdiff() {
        Ok(_) => {},
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        },
    };
}
