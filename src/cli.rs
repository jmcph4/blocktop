use clap::Parser;
use url::Url;

#[derive(Clone, Debug, Parser)]
pub struct Opts {
    #[clap(short, long, default_value = "wss://eth.merkle.io")]
    pub rpc: Url,
}
