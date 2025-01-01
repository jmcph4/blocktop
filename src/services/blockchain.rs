use std::thread::{self, JoinHandle};

use futures::StreamExt;
use tokio::runtime::Builder;
use url::Url;

use crate::{
    client::{Client, WsClient},
    db::Database,
};

const NUM_WORKERS: usize = 1;

#[derive(Clone, Debug)]
pub struct BlockchainService {
    client: WsClient,
}

impl BlockchainService {
    pub fn spawn(rpc: Url, db: Database) -> JoinHandle<eyre::Result<Self>> {
        thread::spawn(move || {
            let runtime = Builder::new_multi_thread()
                .worker_threads(NUM_WORKERS)
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                let this = Self {
                    client: WsClient::new(rpc).await?,
                };
                while let Some(header) =
                    this.client.block_headers().await?.next().await
                {
                    db.add_block_header(&header)?;
                }
                Ok(this)
            })
        })
    }
}
