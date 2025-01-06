use clap::Parser;
use log::warn;

use crate::{
    cli::Opts,
    db::{Database, Location},
    services::blockchain::BlockchainService,
    ui::run,
};

pub mod cli;
pub mod client;
pub mod db;
pub mod services;
pub mod ui;
pub mod utils;

fn main() -> eyre::Result<()> {
    pretty_env_logger::init_timed();
    let opts: Opts = Opts::parse();

    let db: Database = Database::new(match opts.db {
        Some(ref file) => Location::Disk(file.to_path_buf()),
        None => Location::Memory,
    })?;
    let blockchain = BlockchainService::spawn(opts.rpc, db.clone());

    if !opts.headless {
        let terminal = ratatui::init();
        let result = run(terminal, &db);
        ratatui::restore();
        result
    } else {
        if opts.db.is_none() {
            warn!("Headless mode without specifying an on-disk database. All data will be lost on exit.");
        }

        let _ = blockchain.join();
        Ok(())
    }
}
