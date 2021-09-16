extern crate lmdb;

use clap::{App, Arg};
use lmdb::{Cursor, Transaction};
use log::error;
use rayon::prelude::*;
use respdiff::{
    config::Config,
    database::{self, answersdb, metadb, queriesdb},
    dataformat::Report,
    error::Error,
    matcher::{self, Field, FieldMismatches},
    QKey, ServerResponseList,
};

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::convert::TryFrom;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::str::FromStr;

struct Args {
    config: Config,
    datafile: PathBuf,
    envdir: PathBuf,
}

fn parse_args() -> Result<Args, Error> {
    let matches = App::new("Respdiff: Msgdiff")
        .about("find differences between answers")
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
        .arg(
            Arg::with_name("datafile")
                .help("JSON report file")
                .short("d")
                .long("datafile")
                .value_name("FILE")
                .takes_value(true),
        )
        .get_matches();

    let envdir: PathBuf = matches.value_of("ENVDIR").unwrap().into();

    Ok(Args {
        config: {
            let path = matches.value_of("config").unwrap_or("respdiff.cfg");
            let file = File::open(path).map_err(Error::ConfigFile)?;
            let buf = BufReader::new(file);
            serde_ini::from_bufread::<_, Config>(buf).map_err(Error::ConfigRead)?
        },
        datafile: {
            match matches.value_of("datafile") {
                Some(path) => PathBuf::from_str(path).unwrap(),
                None => {
                    let mut path = envdir.clone();
                    path.push("report.json");
                    path
                }
            }
        },
        envdir,
    })
}

fn msgdiff() -> Result<(), Error> {
    let args = parse_args()?;
    let mut report = Report::new();

    let env = match database::open_env(&args.envdir) {
        Ok(env) => env,
        Err(e) => {
            error!("failed to open LMDB environment: {:?}", e);
            std::process::exit(1);
        }
    };

    let adb = database::open_db(&env, answersdb::NAME, false)?;
    {
        let txn = env.begin_ro_txn()?;
        let mut cur = txn.open_ro_cursor(adb)?;

        let reply_lists: Vec<_> = cur
            .iter()
            .map(|res| match res {
                Ok(item) => match ServerResponseList::try_from(item) {
                    Ok(reply_list) => reply_list,
                    Err(e) => {
                        error!("{}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    error!("{}", Error::Database(e));
                    std::process::exit(1);
                }
            })
            .collect();

        // TODO readability: refactor into func
        if args.config.servers.len() < 2 {
            error!("Not enough servers to compare");
            std::process::exit(1);
        }
        let target = &args.config.diff.target;
        let i_target = args
            .config
            .servers
            .iter()
            .position(|x| x == target)
            .ok_or(Error::InvalidServerName)?;
        let i_others = args
            .config
            .servers
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                if s != target {
                    return Some(i);
                }
                None
            })
            .collect::<Vec<_>>();
        let i_cmp_target = (i_others[0], i_target);
        let i_cmps_others: Vec<(usize, usize)> =   // TODO formatting
            i_others
                .iter()
                .copied()
                .zip(
                    i_others
                    .iter()
                    .copied()
                    .skip(1)
                )
                .collect();

        let others_disagreements = reply_lists
            .par_iter()
            .filter_map(|reply_list| {
                assert_eq!(reply_list.replies.len(), args.config.servers.len());
                for (j, k) in &i_cmps_others {
                    let diff = matcher::compare(
                        &reply_list.replies[*j],
                        &reply_list.replies[*k],
                        &args.config.diff.criteria,
                    );
                    if !diff.is_empty() {
                        return Some(reply_list.key);
                    }
                }
                None
            })
            .collect::<BTreeSet<QKey>>();

        let diffs: BTreeMap<_, _> = reply_lists
            .par_iter()
            .filter_map(|reply_list| {
                let diff = matcher::compare(
                    &reply_list.replies[i_cmp_target.0],
                    &reply_list.replies[i_cmp_target.1],
                    &args.config.diff.criteria,
                );
                if !diff.is_empty() {
                    return Some((reply_list.key, diff));
                }
                None
            })
            .collect();

        let mut target_disagreements: BTreeMap<Field, FieldMismatches> = BTreeMap::new();
        for (key, qmismatches) in diffs {
            if others_disagreements.contains(&key) {
                continue;
            }
            for mismatch in qmismatches {
                let field: Field = Field::from(&mismatch);
                let mismatches = match target_disagreements.get_mut(&field) {
                    Some(mismatches) => mismatches,
                    None => {
                        target_disagreements.insert(field, HashMap::new());
                        target_disagreements.get_mut(&field).unwrap()
                    }
                };
                let queries = match mismatches.get_mut(&mismatch) {
                    Some(queries) => queries,
                    None => {
                        mismatches.insert(mismatch.clone(), BTreeSet::new());
                        mismatches.get_mut(&mismatch).unwrap()
                    }
                };
                queries.insert(key);
            }
        }

        report.set_others_disagree(&others_disagreements);
        report.set_target_disagrees(target_disagreements);
    }

    {
        let mdb = database::open_db(&env, metadb::NAME, false)?;
        let txn = env.begin_ro_txn()?;

        report.start_time = metadb::read_start_time(mdb, &txn)?;
        report.end_time = metadb::read_end_time(mdb, &txn)?;
    }
    // TODO is there better way to commit txn besides using a command block?
    {
        let qdb = database::open_db(&env, queriesdb::NAME, false)?;
        let txn = env.begin_ro_txn()?;
        let mut cur = txn.open_ro_cursor(qdb)?;
        report.total_queries = cur.iter().count() as u64;
    }
    {
        let txn = env.begin_ro_txn()?;
        let mut cur = txn.open_ro_cursor(adb)?;
        report.total_answers = cur.iter().count() as u64;
    }

    let out = File::create(args.datafile).map_err(Error::DatafileWrite)?; // TODO maybe check if exists
    serde_json::to_writer(&out, &report)?;

    Ok(())
}

fn main() {
    env_logger::init();

    match msgdiff() {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };
}
