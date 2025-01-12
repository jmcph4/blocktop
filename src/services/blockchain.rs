//! Indexing service for EVM chains
use std::thread::{self, JoinHandle};

use alloy::providers::Provider;
use eyre::eyre;
use futures::StreamExt;
use log::{debug, error};
use tokio::runtime::Builder;
use url::Url;

use crate::{
    client::{AnyClient, Client},
    db::Database,
};

const NUM_WORKERS: usize = 1;

/// Handle to the blockchain indexing service
#[derive(Clone, Debug)]
pub struct BlockchainService {
    client: AnyClient,
}

impl BlockchainService {
    /// Spawn a new instance of the indexing service on its own OS thread
    ///
    /// Connects to the RPC node reachable at the provided [`Url`] and indexes
    /// data to the provided [`Database`].
    ///
    /// Note that joining on the returned thread handle will never yield.
    pub fn spawn(rpc: Url, db: Database) -> JoinHandle<eyre::Result<Self>> {
        thread::spawn(move || {
            let runtime = Builder::new_multi_thread()
                .worker_threads(NUM_WORKERS)
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                let this = Self {
                    client: AnyClient::new(rpc).await?,
                };
                while let Some(header) =
                    this.client.block_headers().await?.next().await
                {
                    let block = this
                        .client
                        .provider()
                        .get_block_by_hash(
                            header.hash,
                            alloy::rpc::types::BlockTransactionsKind::Full,
                        )
                        .await?
                        .ok_or(eyre!("No such block"))?;
                    db.add_block(&block).inspect_err(|e| {
                        error!("Failed to write block to database: {e:?}")
                    })?;
                    debug!("Saved header: {}", &header.hash);
                }
                Ok(this)
            })
        })
    }
}
