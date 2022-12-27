use clap::Parser;
use env_logger::Env;
use log::error;

mod commands;
use commands::{Executable, Respdiff};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();
    let args = Respdiff::parse();
    match args.command.exec(&args) {
        Ok(_) => {}
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };
}
