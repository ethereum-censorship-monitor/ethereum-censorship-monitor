use std::time::SystemTime;

use ethers::{
    prelude::*,
    providers::{Http, Middleware, Provider, Ws},
};
use thiserror::Error;
use tokio::sync::mpsc::Sender;

use crate::{
    config::Config,
    consensus_api::{ConsensusAPIError, ConsensusProvider},
    types::{BeaconBlock, NewBeaconHeadEvent, Timestamp, TxHash, TxpoolContent},
};

const BEACON_GENESIS_ROOT_HEX: &str =
    "4d611d5b93fdab69013a7f0a2f961caca0c853f87cfe9595fe50038163079360";

/// NodeConfig stores the RPC and websocket URLs to an Ethereum node.
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub execution_http_url: url::Url,
    pub execution_ws_url: url::Url,
    pub consensus_http_url: url::Url,
}

impl NodeConfig {
    pub fn from(config: &Config) -> Self {
        NodeConfig {
            execution_http_url: config.execution_http_url.clone(),
            execution_ws_url: config.execution_ws_url.clone(),
            consensus_http_url: config.consensus_http_url.clone(),
        }
    }

    /// Create a provider for the node at http_url.
    pub fn execution_provider(&self) -> Provider<Http> {
        let url = self.execution_http_url.as_str();
        // Unwrapping is fine as try_from only fails with a parse error if url is
        // invalid. Since we just serialized it, we know this is not the case.
        Provider::try_from(url).unwrap()
    }

    /// Create and connect a websocket provider for the node at
    /// execution_ws_url.
    pub async fn ws_provider(&self) -> Result<Provider<Ws>, ProviderError> {
        let url = self.execution_ws_url.as_str();
        Provider::connect(url).await
    }

    /// Create and connect a consensus node provider for the node at
    /// consensus_http_url.
    pub fn consensus_provider(&self) -> ConsensusProvider {
        ConsensusProvider::new(self.consensus_http_url.clone())
    }

    pub async fn test_connection(&self) -> Result<(), WatchError> {
        let p = self.execution_provider();
        p.get_block_number().await?;

        let p = self.consensus_provider();
        let beacon_genesis_root: H256 =
            H256::from_slice(hex::decode(BEACON_GENESIS_ROOT_HEX).unwrap().as_slice());
        p.fetch_beacon_block(beacon_genesis_root).await?;

        let p = self.ws_provider().await?;
        let s = p.subscribe_pending_txs().await?;
        s.unsubscribe().await?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum Event {
    NewTransaction {
        hash: TxHash,
        timestamp: Timestamp,
    },
    NewHead {
        beacon_block: BeaconBlock<Transaction>,
        timestamp: Timestamp,
    },
    TxpoolContent {
        content: TxpoolContent,
        timestamp: Timestamp,
    },
}

#[derive(Error, Debug)]
pub enum WatchError {
    #[error("event stream ended unexpectedly")]
    StreamEnded,
    #[error("failed to send event to channel")]
    Send(#[from] tokio::sync::mpsc::error::SendError<Event>),
    #[error("error from execution client")]
    Provider(#[from] ethers::providers::ProviderError),
    #[error("error joining tasks")]
    Join(#[from] tokio::task::JoinError),
    #[error("error listening to blocks from event source")]
    ReqwestEventsource(#[from] reqwest_eventsource::Error),
    #[error("received invalid JSON data")]
    JSONDecoding {
        data: String,
        source: serde_json::Error,
    },
    #[error("error from consensus client")]
    ConsensusAPI(#[from] ConsensusAPIError),
}

/// Get the current timestamp, i.e. number of seconds since unix epoch.
fn get_current_timestamp() -> Timestamp {
    // unwrapping is fine since now will always be later than the unix epoch
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Watch for relevant events. The events are sent to the given tx channel. The
/// following events are yielded:
///
/// - NewTransaction: whenever we see a new pending transaction
/// - NewHead: whenever we see a new head block
/// - TxpoolContent: after each new head, the tx pool content is queried and
///   yielded
///
/// Returns an error if there's an issue with the node connection or the
/// receiving side of the channel is closed.
pub async fn watch(node_config: &NodeConfig, tx: Sender<Event>) -> Result<(), WatchError> {
    let transactions_handle = tokio::spawn(watch_transactions(node_config.clone(), tx.clone()));
    let heads_handle = tokio::spawn(watch_heads(node_config.clone(), tx.clone()));
    let r = tokio::select! {
        r = transactions_handle => r,
        r = heads_handle => r,
    };
    match r {
        Ok(r) => r,
        Err(e) => Err(WatchError::from(e)),
    }
}

pub async fn watch_transactions(
    node_config: NodeConfig,
    tx: Sender<Event>,
) -> Result<(), WatchError> {
    let ws_provider = node_config.ws_provider().await?;
    let mut stream = ws_provider.subscribe_pending_txs().await?;

    while let Some(hash) = stream.next().await {
        let event = Event::NewTransaction {
            hash,
            timestamp: get_current_timestamp(),
        };

        // send event to channel, but only if it's less than 50% full, drop it
        // otherwise. Block and pool observations are more important, so we make
        // sure there's room for them.
        let relative_capacity = tx.capacity() as f32 / tx.max_capacity() as f32;
        if relative_capacity > 0.5 {
            tx.send(event).await?;
        }
    }
    Err(WatchError::StreamEnded)
}

async fn watch_heads(node_config: NodeConfig, tx: Sender<Event>) -> Result<(), WatchError> {
    let exec_provider = node_config.execution_provider();
    let cons_provider = node_config.consensus_provider();

    let url = node_config
        .consensus_http_url
        .join("/eth/v1/events?topics=head")
        .unwrap();
    let request = reqwest::Client::new().get(url);
    let mut es = reqwest_eventsource::EventSource::new(request).unwrap();
    while let Some(event) = es.next().await {
        let t = get_current_timestamp();
        match event {
            Ok(reqwest_eventsource::Event::Open) => {}
            Ok(reqwest_eventsource::Event::Message(message)) => {
                let event: Result<NewBeaconHeadEvent, serde_json::Error> =
                    serde_json::from_str(message.data.as_str());
                if let Err(e) = event {
                    es.close();
                    return Err(WatchError::JSONDecoding {
                        source: e,
                        data: message.data,
                    });
                }
                let event = event.unwrap();

                let beacon_block_without_root = cons_provider.fetch_beacon_block(event.block).await;
                if let Err(e) = beacon_block_without_root {
                    es.close();
                    return Err(WatchError::from(e));
                }
                let beacon_block =
                    BeaconBlock::new(beacon_block_without_root.unwrap(), event.block);

                let relative_capacity = tx.capacity() as f32 / tx.max_capacity() as f32;
                if relative_capacity < 0.1 {
                    log::warn!("event channel is getting full, blocks might arrive late");
                }

                if let Err(e) = tx
                    .send(Event::NewHead {
                        beacon_block,
                        timestamp: t,
                    })
                    .await
                {
                    es.close();
                    return Err(WatchError::from(e));
                }
            }
            Err(e) => {
                es.close();
                return Err(WatchError::from(e));
            }
        }

        let content = exec_provider.txpool_content().await?;
        let event = Event::TxpoolContent {
            content,
            timestamp: get_current_timestamp(),
        };
        tx.send(event).await?;
    }
    Err(WatchError::StreamEnded)
}
