use std::path::PathBuf;

use alloy::{eips::BlockHashOrNumber, primitives::TxHash};
use clap::Parser;
use url::Url;

pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_METRICS_ONLY_PORT: u16 = 8080;

/// Minimalist TUI block explorer and chain indexer
#[derive(Clone, Debug, Parser)]
#[clap(version, about, author)]
pub struct Opts {
    #[clap(short, long, default_value = "wss://eth.merkle.io")]
    pub rpc: Url,
    #[clap(short, long)]
    pub db: Option<PathBuf>,
    #[clap(long, action)]
    pub headless: bool,
    #[clap(long, action)]
    pub list_block_hashes: bool,
    #[clap(long)]
    pub block: Option<BlockHashOrNumber>,
    #[clap(long, alias("tx"))]
    pub transaction: Option<TxHash>,
    #[clap(long, short, action)]
    pub serve: bool,
    #[clap(long, short, action)]
    pub metrics: bool,
    #[clap(long, short)]
    pub port: Option<u16>,
}

impl Opts {
    pub fn port(&self) -> Option<u16> {
        if let Some(port) = self.port {
            Some(port)
        } else {
            match (self.serve, self.metrics) {
                (true, true) => Some(DEFAULT_PORT),
                (false, true) => Some(DEFAULT_METRICS_ONLY_PORT),
                (true, false) => Some(DEFAULT_PORT),
                (false, false) => None,
            }
        }
    }
}
