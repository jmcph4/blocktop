#![allow(async_fn_in_trait)]
use std::sync::Arc;

use alloy::{
    primitives::ChainId,
    providers::{Provider, ProviderBuilder, RootProvider, WsConnect},
    pubsub::PubSubFrontend,
    rpc::types::{Block, Header, Transaction},
};
use futures::Stream;
use log::{debug, info};
use url::Url;

pub trait Client {
    fn url(&self) -> Url;
    fn chain_id(&self) -> ChainId;
    async fn blocks(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Block> + Unpin>>;
    async fn block_headers(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Header> + Unpin>>;
    async fn pending_transactions(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Transaction> + Unpin>>;
}

#[derive(Clone, Debug)]
pub struct WsClient {
    url: Url,
    chain_id: ChainId,
    provider: Arc<RootProvider<PubSubFrontend>>,
}

impl WsClient {
    pub async fn new(url: Url) -> eyre::Result<Self> {
        let provider = Arc::new(
            ProviderBuilder::new()
                .on_ws(WsConnect::new(url.clone()))
                .await?,
        );
        let chain_id = provider.get_chain_id().await?;
        info!(
            "Client initialised (endpoint: {}, chain: {})",
            url, chain_id
        );
        Ok(Self {
            url,
            chain_id,
            provider,
        })
    }

    pub fn provider(&self) -> &RootProvider<PubSubFrontend> {
        &self.provider
    }
}

impl Client for WsClient {
    fn url(&self) -> Url {
        self.url.clone()
    }

    fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    async fn blocks(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Block> + Unpin>> {
        debug!("Subscribing to block stream...");
        todo!()
    }

    async fn block_headers(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Header> + Unpin>> {
        debug!("Subscribing to block header stream...");
        Ok(Box::new(
            self.provider.subscribe_blocks().await?.into_stream(),
        ))
    }

    async fn pending_transactions(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Transaction> + Unpin>> {
        debug!("Subscribing to pending transaction stream...");
        Ok(Box::new(
            self.provider
                .subscribe_full_pending_transactions()
                .await?
                .into_stream(),
        ))
    }
}
