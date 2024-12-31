use clap::Parser;

use crate::cli::Opts;
use crate::ui::run;

pub mod cli;
pub mod ui;

fn main() -> eyre::Result<()> {
    pretty_env_logger::init_timed();
    let _opts: Opts = Opts::parse();

    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();

    result
}
