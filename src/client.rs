//! Blockchain client communications
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

/// Interface to an Ethereum node
pub trait Client {
    /// The URL of the endpoint that this client is connected to
    fn url(&self) -> Url;
    /// The [`ChainId`] that this client is on
    fn chain_id(&self) -> ChainId;
    /// Subscription stream yielding full [`Block`]s
    async fn blocks(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Block> + Unpin>>;
    /// Subscription stream yielding only block [`Header`]s
    async fn block_headers(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Header> + Unpin>>;
    /// Subscription stream yielding pending transactions from the mempool
    async fn pending_transactions(
        &self,
    ) -> eyre::Result<Box<dyn Stream<Item = Transaction> + Unpin>>;
    /// Retrieve the [`Block`] associated with the given identifier
    async fn block(&self, tag: BlockNumberOrTag) -> eyre::Result<Block>;
}

/// Client type that is generic over all supported transports
#[derive(Clone, Debug)]
pub enum AnyClient {
    /// Websockets
    Ws(WsClient),
    /// IPC (Unix sockets)
    Ipc(IpcClient),
}

impl AnyClient {
    /// Parse the provided [`Url`] into the corresponding [`AnyClient`]
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

    /// Handle to the internal Alloy provider
    pub fn provider(&self) -> &RootProvider<PubSubFrontend> {
        match self {
            Self::Ws(t) => t.provider(),
            Self::Ipc(t) => t.provider(),
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

/// Websocket client
#[derive(Clone, Debug)]
pub struct WsClient {
    url: Url,
    chain_id: ChainId,
    provider: Arc<RootProvider<PubSubFrontend>>,
}

impl WsClient {
    /// Produce a handle to a Websocket client given a [`Url`]
    ///
    /// This will query the [`ChainId`] upon successful connection to the node.
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
    /// Produce a handle to an IPC client given a filepath to a Unix named pipe
    ///
    /// This will query the [`ChainId`] upon successful connection to the node.
    /// Note that this path does **not** contain an `ipc://` URI scheme prefix.
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

    /// Handle to the internal Alloy provider
    pub fn provider(&self) -> &RootProvider<PubSubFrontend> {
        &self.provider
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
