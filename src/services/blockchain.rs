use std::{
    io::Write,
    thread::{self, JoinHandle},
};

use futures::StreamExt;
use tokio::runtime::Builder;
use url::Url;

use alloy::primitives::B256;

use crate::client::{Client, WsClient};

const NUM_WORKERS: usize = 1;

#[derive(Clone, Debug, Default)]
pub struct HeadInfo {
    pub latest_block_hash: Option<B256>,
}

#[derive(Clone, Debug)]
pub struct BlockchainService {
    client: WsClient,
    info: HeadInfo,
}

impl BlockchainService {
    pub fn spawn(rpc: Url) -> JoinHandle<eyre::Result<Self>> {
        thread::spawn(|| {
            let runtime = Builder::new_multi_thread()
                .worker_threads(NUM_WORKERS)
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                let mut this = Self {
                    client: WsClient::new(rpc).await?,
                    info: HeadInfo::default(),
                };
                while let Some(header) =
                    this.client.block_headers().await?.next().await
                {
                    this.info.latest_block_hash = Some(header.hash);
                    std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open("headers.txt")
                        .unwrap()
                        .write_all(
                            format!("block: {}\n", header.hash).as_bytes(),
                        )
                        .unwrap();
                }
                Ok(this)
            })
        })
    }

    pub fn latest_block_hash(&self) -> Option<B256> {
        self.info.latest_block_hash
    }
}
