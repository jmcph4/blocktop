use clap::Parser;

use crate::cli::Opts;

pub mod cli;

fn main() {
    pretty_env_logger::init_timed();
    let _opts: Opts = Opts::parse();
}
