use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use lmdb::Environment;

use std::env;
use std::path::PathBuf;

use respdiff::config::Config;
use respdiff::database;

mod diff_answers;
mod transceive;

pub trait Executable {
    fn exec(&self, args: &Respdiff) -> Result<()>;
}

/// DNS response differencing toolchain.
#[derive(Debug, Parser)]
#[clap(name = "respdiff", version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Respdiff {
    #[clap(flatten)]
    global_opts: GlobalOpts,

    #[command(subcommand)]
    pub command: Command,
}
impl Respdiff {
    pub fn config(&self) -> Result<Config> {
        Ok(Config::try_from(&self.global_opts.config)?)
    }
    pub fn envdir(&self) -> Result<PathBuf> {
        match &self.global_opts.envdir {
            Some(envdir) => Ok(envdir.clone()),
            None => Ok(env::current_dir()?),
        }
    }
    pub fn env(&self) -> Result<Environment> {
        let path = self.envdir()?;
        Ok(database::open_env(&path)?)
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Find differences between answers.
    DiffAnswers(diff_answers::DiffAnswers),
    /// Send queries to servers and record answers.
    Transceive(transceive::Transceive),
}
impl Executable for Command {
    fn exec(&self, args: &Respdiff) -> Result<()> {
        use Command::*;
        match self {
            DiffAnswers(cmd) => cmd.exec(args),
            Transceive(cmd) => cmd.exec(args),
        }
    }
}

#[derive(Debug, Args)]
struct GlobalOpts {
    /// Configuration file path.
    #[arg(short, long, value_name = "FILE", global = true)]
    config: Option<PathBuf>,

    /// LMDB environment directory.
    #[arg(short, long, value_name = "DIR", global = true)]
    envdir: Option<PathBuf>,
}
