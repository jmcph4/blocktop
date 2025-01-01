use clap::Parser;
use services::blockchain::BlockchainService;

use crate::{cli::Opts, ui::run};

pub mod cli;
pub mod client;
pub mod services;
pub mod ui;

fn main() -> eyre::Result<()> {
    pretty_env_logger::init_timed();
    let opts: Opts = Opts::parse();

    let _blockchain = BlockchainService::spawn(opts.rpc);

    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();

    result
}
