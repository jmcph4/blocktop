use clap::Parser;

use crate::cli::Opts;

pub mod cli;

fn main() {
    let _opts: Opts = Opts::parse();
}
