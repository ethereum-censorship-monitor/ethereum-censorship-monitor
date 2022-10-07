use crate::types::{Block, Timestamp, TxHash, TxpoolContent, H256};
use ethers::{
    prelude::*,
    providers::{Http, Middleware, Provider, Ws},
};
use std::fmt;
use std::time::SystemTime;
use tokio::sync::mpsc::Sender;

/// NodeConfig stores the RPC and websocket URLs to an Ethereum node.
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub http_url: url::Url,
    pub ws_url: url::Url,
}

impl NodeConfig {
    /// Create a provider for the node at http_url.
    pub fn http_provider(&self) -> Provider<Http> {
        let url = self.http_url.as_str();
        // Unwrapping is fine as try_from only fails with a parse error if url is invalid. Since we
        // just serialized it, we know this is not the case.
        Provider::try_from(url).unwrap()
    }

    /// Create and connect a websocket provider for the node at ws_url.
    pub async fn ws_provider(&self) -> Result<Provider<Ws>, WatchError> {
        let url = self.ws_url.as_str();
        Provider::connect(url)
            .await
            .map_err(WatchError::ProviderError)
    }
}

#[derive(Debug)]
pub enum Event {
    NewTransaction {
        hash: TxHash,
        timestamp: Timestamp,
    },
    NewHead {
        block: Block<H256>,
        timestamp: Timestamp,
    },
    TxpoolContent {
        content: TxpoolContent,
        timestamp: Timestamp,
    },
}

impl Event {
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Event::NewTransaction {
                hash: _,
                timestamp: t,
            } => *t,
            Event::NewHead {
                block: _,
                timestamp: t,
            } => *t,
            Event::TxpoolContent {
                content: _,
                timestamp: t,
            } => *t,
        }
    }
}

#[derive(Debug)]
pub enum WatchError {
    StreamEndedError,
    SendError(tokio::sync::mpsc::error::SendError<Event>),
    ProviderError(ethers::providers::ProviderError),
    JoinError(tokio::task::JoinError),
}

impl fmt::Display for WatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WatchError::StreamEndedError => self.fmt(f),
            WatchError::SendError(_) => self.fmt(f),
            WatchError::ProviderError(_) => self.fmt(f),
            WatchError::JoinError(_) => self.fmt(f),
        }
    }
}

impl std::error::Error for WatchError {}

#[derive(Debug)]
struct StreamEndedError;

impl fmt::Display for StreamEndedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "stream ended unexpectedly")
    }
}

impl std::error::Error for StreamEndedError {}

/// Get the current timestamp, i.e. number of seconds since unix epoch.
fn get_current_timestamp() -> Timestamp {
    // unwrapping is fine since now will always be later than the unix epoch
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Watch for relevant events. The events are sent to the given tx channel. The following events are
/// yielded:
///
/// - NewTransaction: whenever we see a new pending transaction
/// - NewHead: whenever we see a new head block
/// - TxpoolContent: after each new head, the tx pool content is queried and yielded
///
/// Returns an error if there's an issue with the node connection or the receiving side of the
/// channel is closed.
pub async fn watch(node_config: NodeConfig, tx: Sender<Event>) -> Result<(), WatchError> {
    let transactions_handle = tokio::spawn(watch_transactions(node_config.clone(), tx.clone()));
    let heads_handle = tokio::spawn(watch_heads(node_config.clone(), tx.clone()));
    let r = tokio::select! {
        r = transactions_handle => r,
        r = heads_handle => r,
    };
    match r {
        Ok(r) => r,
        Err(e) => Err(WatchError::JoinError(e)),
    }
}

pub async fn watch_transactions(
    node_config: NodeConfig,
    tx: Sender<Event>,
) -> Result<(), WatchError> {
    let ws_provider = node_config.ws_provider().await?;
    let mut stream = ws_provider
        .subscribe_pending_txs()
        .await
        .map_err(WatchError::ProviderError)?;

    while let Some(hash) = stream.next().await {
        let event = Event::NewTransaction {
            hash,
            timestamp: get_current_timestamp(),
        };
        tx.send(event).await.map_err(WatchError::SendError)?;
    }
    Err(WatchError::StreamEndedError)
}

pub async fn watch_heads(node_config: NodeConfig, tx: Sender<Event>) -> Result<(), WatchError> {
    let http_provider = node_config.http_provider();
    let ws_provider = node_config.ws_provider().await?;

    let mut block_stream = ws_provider
        .subscribe_blocks()
        .await
        .map_err(WatchError::ProviderError)?;

    while let Some(block) = block_stream.next().await {
        let event = Event::NewHead {
            block,
            timestamp: get_current_timestamp(),
        };
        tx.send(event).await.map_err(WatchError::SendError)?;

        let content = http_provider
            .txpool_content()
            .await
            .map_err(WatchError::ProviderError)?;
        let event = Event::TxpoolContent {
            content,
            timestamp: get_current_timestamp(),
        };
        tx.send(event).await.map_err(WatchError::SendError)?;
    }
    Err(WatchError::StreamEndedError)
}
