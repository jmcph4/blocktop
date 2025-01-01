use clap::Parser;
use db::{Database, Location};
use services::blockchain::BlockchainService;

use crate::{cli::Opts, ui::run};

pub mod cli;
pub mod client;
pub mod db;
pub mod services;
pub mod ui;

fn main() -> eyre::Result<()> {
    pretty_env_logger::init_timed();
    let opts: Opts = Opts::parse();

    let db: Database = Database::new(match opts.db {
        Some(file) => Location::Disk(file),
        None => Location::Memory,
    })?;
    let _blockchain = BlockchainService::spawn(opts.rpc, db.clone());

    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();

    result
}
