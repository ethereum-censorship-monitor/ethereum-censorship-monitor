use ethers::utils::keccak256;
use rlp::Decodable;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

use crate::types::{BeaconBlockWithoutRoot, ConsensusSyncStatus, SignedMessage, Transaction, H256};

#[derive(Error, Debug)]
pub enum ConsensusAPIError {
    #[error("error fetching {requested}")]
    ReqwestError {
        source: reqwest::Error,
        requested: String,
    },
    #[error("unexpected node response: {description}")]
    UnexpectedResponse { description: String },
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ConsensusAPIResponse<T> {
    pub data: T,
    pub execution_optimistic: Option<bool>,
}

#[derive(Debug)]
pub struct ConsensusProvider {
    http_url: Url,
}

impl ConsensusProvider {
    pub fn new(http_url: Url) -> Self {
        ConsensusProvider { http_url }
    }

    pub async fn fetch_beacon_block_by_root(
        &self,
        root: H256,
    ) -> Result<BeaconBlockWithoutRoot<Transaction>, ConsensusAPIError> {
        let path = format!("0x{}", hex::encode(root));
        self.fetch_beacon_block_with_path(path).await
    }

    pub async fn fetch_beacon_block_by_slot(
        &self,
        slot: u64,
    ) -> Result<BeaconBlockWithoutRoot<Transaction>, ConsensusAPIError> {
        let path = slot.to_string();
        self.fetch_beacon_block_with_path(path).await
    }

    async fn fetch_beacon_block_with_path(
        &self,
        path: String,
    ) -> Result<BeaconBlockWithoutRoot<Transaction>, ConsensusAPIError> {
        let url = self
            .http_url
            .join(format!("/eth/v2/beacon/blocks/{}", path).as_str())
            .unwrap();

        let r = reqwest::get(url)
            .await
            .map_err(|e| ConsensusAPIError::ReqwestError {
                source: e,
                requested: String::from("beacon block"),
            })?
            .error_for_status()
            .map_err(|e| ConsensusAPIError::ReqwestError {
                source: e,
                requested: String::from("beacon block"),
            })?;
        let response: ConsensusAPIResponse<SignedMessage<BeaconBlockWithoutRoot<String>>> = r
            .json()
            .await
            .map_err(|e| ConsensusAPIError::ReqwestError {
                source: e,
                requested: String::from("beacon block"),
            })?;

        if response.execution_optimistic.unwrap_or(false) {
            return Err(ConsensusAPIError::UnexpectedResponse {
                description: String::from("consensus API response is optimistic"),
            });
        }

        let tx_strings = &response.data.message.body.execution_payload.transactions;
        let mut txs = Vec::new();
        for s in tx_strings {
            let b = hex::decode(s.strip_prefix("0x").unwrap_or(s.as_str())).map_err(|e| {
                ConsensusAPIError::UnexpectedResponse {
                    description: format!("error decoding tx in block: {}", e),
                }
            })?;
            let tx = Transaction::decode(&rlp::Rlp::new(b.as_slice()));
            match tx {
                Err(e) => log::warn!(
                    "received block {} with undecodable tx 0x{}: {}",
                    response.data.message,
                    hex::encode(keccak256(b)),
                    e,
                ),
                Ok(mut tx) => {
                    // set tx hash manually (see https://github.com/gakonst/ethers-rs/issues/1849)
                    tx.hash = H256::from(keccak256(b));
                    txs.push(tx);
                }
            }
        }
        let beacon_block = BeaconBlockWithoutRoot::with_transactions(response.data.message, txs);
        Ok(beacon_block)
    }

    pub async fn fetch_sync_status(&self) -> Result<ConsensusSyncStatus, ConsensusAPIError> {
        let url = self.http_url.join("/eth/v1/node/syncing").unwrap();
        let r = reqwest::get(url)
            .await
            .map_err(|e| ConsensusAPIError::ReqwestError {
                source: e,
                requested: String::from("sync status"),
            })?
            .error_for_status()
            .map_err(|e| ConsensusAPIError::ReqwestError {
                source: e,
                requested: String::from("sync status"),
            })?;
        let response: ConsensusAPIResponse<ConsensusSyncStatus> =
            r.json()
                .await
                .map_err(|e| ConsensusAPIError::ReqwestError {
                    source: e,
                    requested: String::from("sync status"),
                })?;
        Ok(response.data)
    }
}
