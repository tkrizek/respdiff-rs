extern crate lmdb;

use anyhow::Result;
use async_std::{prelude::*, task};
use byteorder::{ByteOrder, LittleEndian};
use clap::Args;
use futures::channel::mpsc;
use lmdb::{Transaction, WriteFlags};
use log::warn;
use respdiff::{
    database::{self, queriesdb},
    transceive,
};

use std::time::Duration;

use crate::commands::{Executable, Respdiff};

#[derive(Debug, Args)]
pub struct Transceive {}

impl Executable for Transceive {
    fn exec(&self, args: &Respdiff) -> Result<()> {
        warn!("SUBCOMMAND transceive IS AN UNFINISHED PROTOTYPE!");
        let config = args.config()?;
        let env = args.env()?;
        let metadb = database::open_db(&env, database::metadb::NAME, true)?;
        {
            let mut txn = env.begin_rw_txn()?;
            database::metadb::write_servers(metadb, &mut txn, config.servers.clone())?;
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

        let servers = config
            .servers
            .iter()
            .map(|name| *config.server_data.get(name).unwrap())
            .collect();
        let (rsender, mut rreceiver) = mpsc::unbounded();
        task::spawn(transceive::send_loop(
            queries,
            servers,
            rsender,
            Duration::from_secs_f64(config.sendrecv.timeout),
            800, // TODO hardcoded qps
        ));

        let adb = database::open_db(&env, database::answersdb::NAME, true)?;
        let mut txn = env.begin_rw_txn()?;
        task::block_on(async move {
            while let Some(responselist) = rreceiver.next().await {
                let key = responselist.key;
                let mut key_buf = [0; 4];
                LittleEndian::write_u32(&mut key_buf, key);
                let data: Vec<u8> = responselist.into();
                txn.put(adb, &key_buf, &data, WriteFlags::empty()).unwrap(); // TODO error handling?
            }
            txn.commit().unwrap(); // TODO err handle?
        });

        let mut txn = env.begin_rw_txn()?;
        database::metadb::write_end_time(metadb, &mut txn)?;
        txn.commit()?;

        Ok(())
    }
}
