extern crate lmdb;

use anyhow::{anyhow, Result};
use clap::Args;
use lmdb::{Cursor, Transaction};
use log::error;
use rayon::prelude::*;
use respdiff::{
    database::{self, answersdb, metadb, queriesdb},
    dataformat::Report,
    matcher::{self, Field, FieldMismatches, Mismatch},
    QKey,
};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::path::PathBuf;

use crate::commands::{Executable, Respdiff};

#[derive(Debug, Args)]
pub struct DiffAnswers {
    /// Path to JSON datafile.
    #[arg(short, long, value_name = "FILE")]
    datafile: Option<PathBuf>,
}

type IndexPair = (usize, usize);

/// Returns indices which are used to compare responses in list.
///
/// Two values are returned.
/// First is a tuple of target index and one of the other servers to
/// compare the answer to.
/// The second returned value is a vector of tuples each with two indicies -- two servers to be
/// compared for equality between each other.
fn indices_to_cmp(target: &str, servers: &[String]) -> Result<(IndexPair, Vec<IndexPair>)> {
    let i_target = servers
        .iter()
        .position(|x| x == target)
        .ok_or_else(|| anyhow!("invalid server name"))?;

    let i_others = servers
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
    let i_cmps_others: Vec<(usize, usize)> = i_others
        .iter()
        .copied()
        .zip(i_others.iter().copied().skip(1))
        .collect();

    Ok((i_cmp_target, i_cmps_others))
}

fn target_disagreements_from_diffs(
    diffs: BTreeMap<QKey, HashSet<Mismatch>>,
    others_disagreements: &BTreeSet<QKey>,
) -> BTreeMap<Field, FieldMismatches> {
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
    target_disagreements
}

impl Executable for DiffAnswers {
    fn exec(&self, args: &Respdiff) -> Result<()> {
        let mut report = Report::new();
        let config = args.config()?;

        if config.servers.len() < 2 {
            error!("Not enough servers to compare");
            std::process::exit(1);
        }

        let datafile = match &self.datafile {
            Some(path) => path.clone(),
            None => {
                let mut path = args.envdir()?;
                path.push("report.json");
                path
            }
        };

        let env = args.env()?;
        let qdb = database::open_db(&env, queriesdb::NAME, false)?;
        let adb = database::open_db(&env, answersdb::NAME, false)?;
        let mdb = database::open_db(&env, metadb::NAME, false)?;
        let txn = env.begin_ro_txn()?;

        let response_lists = answersdb::get_response_lists(adb, &txn)?;
        let (i_cmp_target, i_cmps_others) = indices_to_cmp(&config.diff.target, &config.servers)?;

        // compare other servers to each other and find their differences
        let others_disagreements = response_lists
            .par_iter()
            .filter_map(|response_list| {
                assert_eq!(response_list.replies.len(), config.servers.len());
                for (j, k) in &i_cmps_others {
                    let diff = matcher::compare(
                        &response_list.replies[*j],
                        &response_list.replies[*k],
                        &config.diff.criteria,
                    );
                    if !diff.is_empty() {
                        return Some(response_list.key);
                    }
                }
                None
            })
            .collect::<BTreeSet<QKey>>();

        // find differences between the target and one of the other servers
        let diffs: BTreeMap<_, _> = response_lists
            .par_iter()
            .filter_map(|response_list| {
                let diff = matcher::compare(
                    &response_list.replies[i_cmp_target.0],
                    &response_list.replies[i_cmp_target.1],
                    &config.diff.criteria,
                );
                if !diff.is_empty() {
                    return Some((response_list.key, diff));
                }
                None
            })
            .collect();

        let target_disagreements = target_disagreements_from_diffs(diffs, &others_disagreements);

        report.set_others_disagree(&others_disagreements);
        report.set_target_disagrees(target_disagreements);
        report.start_time = metadb::read_start_time(mdb, &txn)?;
        report.end_time = metadb::read_end_time(mdb, &txn)?;

        let mut cur = txn.open_ro_cursor(qdb)?;
        report.total_queries = cur.iter().count() as u64;

        let mut cur = txn.open_ro_cursor(adb)?;
        report.total_answers = cur.iter().count() as u64;

        let out = File::create(datafile)?;
        serde_json::to_writer(&out, &report)?;

        Ok(())
    }
}
