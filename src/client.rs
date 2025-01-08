#![allow(async_fn_in_trait)]
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use alloy::{
    eips::BlockNumberOrTag,
    primitives::ChainId,
    providers::{
        IpcConnect, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    pubsub::{PubSubConnect, PubSubFrontend},
    rpc::types::{Block, Header, Transaction},
};
use eyre::eyre;
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
    async fn block(&self, tag: BlockNumberOrTag) -> eyre::Result<Block>;
}

#[derive(Clone, Debug)]
pub enum AnyClient {
    Ws(WsClient),
    Ipc(IpcClient),
}

impl AnyClient {
    pub async fn new(url: Url) -> eyre::Result<Self> {
        match url.scheme() {
            "ws" | "wss" => Ok(AnyClient::Ws(WsClient::new(url).await?)),
            "ipc" => Ok(AnyClient::Ipc(
                IpcClient::new::<PathBuf>(
                    url.to_string().strip_prefix("ipc://").unwrap().into(),
                )
                .await?,
            )),
            _ => Err(eyre!("Unsupported URL scheme")),
        }
    }
}

impl Client for AnyClient {
    fn url(&self) -> Url {
        match self {
            Self::Ws(t) => t.url(),
            Self::Ipc(t) => t.url(),
        }
    }

    fn chain_id(&self) -> ChainId {
        match self {
            Self::Ws(t) => t.chain_id(),
            Self::Ipc(t) => t.chain_id(),
        }
    }

    async fn blocks(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Block> + Unpin>> {
        Ok(match self {
            Self::Ws(t) => t.blocks().await?,
            Self::Ipc(t) => t.blocks().await?,
        })
    }

    async fn block_headers(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Header> + Unpin>> {
        Ok(match self {
            Self::Ws(t) => t.block_headers().await?,
            Self::Ipc(t) => t.block_headers().await?,
        })
    }

    async fn pending_transactions(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Transaction> + Unpin>> {
        Ok(match self {
            Self::Ws(t) => t.pending_transactions().await?,
            Self::Ipc(t) => t.pending_transactions().await?,
        })
    }

    async fn block(&self, tag: BlockNumberOrTag) -> eyre::Result<Block> {
        Ok(match self {
            Self::Ws(t) => t.block(tag).await?,
            Self::Ipc(t) => t.block(tag).await?,
        })
    }
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
            "Websockets client initialised (endpoint: {}, chain: {})",
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

    async fn block(&self, tag: BlockNumberOrTag) -> eyre::Result<Block> {
        debug!("Retrieving block {}...", tag);
        match self
            .provider
            .get_block(
                tag.into(),
                alloy::rpc::types::BlockTransactionsKind::Full,
            )
            .await?
        {
            Some(t) => Ok(t),
            None => Err(eyre!("No block")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct IpcClient {
    path: PathBuf,
    chain_id: ChainId,
    provider: Arc<RootProvider<PubSubFrontend>>,
}

impl IpcClient {
    pub async fn new<P: AsRef<Path> + Clone>(path: P) -> eyre::Result<Self>
    where
        IpcConnect<P>: PubSubConnect,
    {
        let ipc = IpcConnect::new(path.clone());
        let provider = Arc::new(ProviderBuilder::new().on_ipc(ipc).await?);
        let chain_id = provider.get_chain_id().await?;
        info!(
            "IPC client initialised (endpoint: ipc://{}, chain: {})",
            path.clone().as_ref().display(),
            chain_id
        );
        Ok(Self {
            path: path.as_ref().into(),
            chain_id,
            provider,
        })
    }
}

impl Client for IpcClient {
    fn url(&self) -> Url {
        format!("ipc://{}", self.path.display()).parse().unwrap()
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

    async fn block(&self, tag: BlockNumberOrTag) -> eyre::Result<Block> {
        debug!("Retrieving block {}...", tag);
        match self
            .provider
            .get_block(
                tag.into(),
                alloy::rpc::types::BlockTransactionsKind::Full,
            )
            .await?
        {
            Some(t) => Ok(t),
            None => Err(eyre!("No block")),
        }
    }
}
