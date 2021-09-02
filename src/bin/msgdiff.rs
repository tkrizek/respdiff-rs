extern crate lmdb;

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;
use clap::{Arg, App};
use lmdb::{Cursor, Transaction};
use log::error;
use rayon::prelude::*;
use respdiff::{self, config::Config, database::{self, answersdb::ServerReplyList}, matcher};
use serde_ini;

struct Args {
    config: Config,
    datafile: PathBuf,
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
        .arg(Arg::with_name("datafile")
            .help("JSON report file")
            .short("d")
            .long("datafile")
            .value_name("FILE")
            .takes_value(true))
        .get_matches();

    let envdir: PathBuf = matches.value_of("ENVDIR").unwrap().into();

    Ok(Args {
        config: {
            let path = matches.value_of("config").unwrap_or("respdiff.cfg");
            let file = File::open(path).map_err(respdiff::Error::ConfigFile)?;
            let buf = BufReader::new(file);
            serde_ini::from_bufread::<_, Config>(buf).map_err(respdiff::Error::ConfigRead)?
        },
        datafile: {
            match matches.value_of("datafile") {
                Some(path) => PathBuf::from_str(path).unwrap(),
                None => {
                    let mut path = envdir.clone();
                    path.push("report.json");
                    path
                },
            }
        },
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

        let reply_lists: Vec<_>= cur.iter().map(|res| {
            match res {
                Ok(item) => {
                    match ServerReplyList::try_from(item) {
                        Ok(reply_list) => reply_list,
                        Err(e) => {
                            error!("{}", e);
                            std::process::exit(1);
                        }
                    }
                },
                Err(e) => {
                    error!("{}", respdiff::Error::Database(e));
                    std::process::exit(1);
                }
            }
        }).collect();
        let diffs: BTreeMap<_, _> = reply_lists.par_iter().filter_map(|reply_list| {
            if reply_list.replies.len() >= 2 {
                let diff = matcher::compare(
                    &reply_list.replies[0],
                    &reply_list.replies[1],
                    &args.config.diff.criteria);
                if diff.len() > 0 {
                    return Some((reply_list.key, diff));
                }
            }
            None
        }).collect();
        for (key, diff) in diffs {
            println!("{} -> {:?}", key, diff);
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
