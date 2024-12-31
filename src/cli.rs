use clap::Parser;
use url::Url;

#[derive(Clone, Debug, Parser)]
pub struct Opts {
    #[clap(short, long)]
    pub rpc: Url,
}
