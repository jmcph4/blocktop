#![allow(async_fn_in_trait)]
use std::sync::Arc;

use alloy::{
    providers::{Provider, ProviderBuilder, RootProvider, WsConnect},
    pubsub::PubSubFrontend,
    rpc::types::{Block, Header, Transaction},
};
use futures::Stream;
use url::Url;

pub trait Client {
    fn url(&self) -> Url;
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
    provider: Arc<RootProvider<PubSubFrontend>>,
}

impl WsClient {
    pub async fn new(url: Url) -> eyre::Result<Self> {
        Ok(Self {
            url: url.clone(),
            provider: Arc::new(
                ProviderBuilder::new().on_ws(WsConnect::new(url)).await?,
            ),
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

    async fn blocks(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Block> + Unpin>> {
        todo!()
    }

    async fn block_headers(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Header> + Unpin>> {
        Ok(Box::new(
            self.provider.subscribe_blocks().await?.into_stream(),
        ))
    }

    async fn pending_transactions(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Transaction> + Unpin>> {
        Ok(Box::new(
            self.provider
                .subscribe_full_pending_transactions()
                .await?
                .into_stream(),
        ))
    }
}
