use crate::types::{BeaconBlockWithoutRoot, Bytes, SignedMessage, Transaction, H256, U256};
use hex;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum ConsensusAPIError {
    #[error("{0}")]
    ReqwestError(reqwest::Error),
    #[error("unexpected response: {description}")]
    UnexpectedResponse { description: String },
}

#[derive(Deserialize, Debug, PartialEq, Clone)]
pub struct ConsensusAPIResponse<T> {
    pub data: T,
    pub execution_optimistic: bool,
}

#[derive(Debug)]
pub struct ConsensusProvider {
    http_url: Url,
}

impl ConsensusProvider {
    pub fn new(http_url: Url) -> Self {
        ConsensusProvider { http_url }
    }

    pub async fn fetch_beacon_block(
        &self,
        root: H256,
    ) -> Result<BeaconBlockWithoutRoot<Transaction>, ConsensusAPIError> {
        let url = self
            .http_url
            .join(format!("/eth/v1/beacon/blocks/0x{}", hex::encode(root)).as_str())
            .unwrap();
        let r = reqwest::get(url)
            .await
            .map_err(ConsensusAPIError::ReqwestError)?
            .error_for_status()
            .map_err(ConsensusAPIError::ReqwestError)?;
        let response: ConsensusAPIResponse<SignedMessage<BeaconBlockWithoutRoot<String>>> =
            r.json().await.map_err(ConsensusAPIError::ReqwestError)?;

        if response.execution_optimistic {
            return Err(ConsensusAPIError::UnexpectedResponse {
                description: String::from("consensus API response is optimistic"),
            });
        }

        let tx_strings = &response.data.message.body.execution_payload.transactions;
        let mut txs = Vec::new();
        for s in tx_strings {
            let b = hex::decode(s.strip_prefix("0x").unwrap_or(s.as_str())).map_err(|e| {
                ConsensusAPIError::UnexpectedResponse {
                    description: String::from(format!("error decoding tx in block: {}", e)),
                }
            })?;
            let tx = rlp::decode(b.as_slice());
            match tx {
                Err(e) => log::warn!(
                    "received block {} with undecodable tx: {}",
                    response.data.message,
                    e
                ),
                Ok(tx) => txs.push(tx),
            }
        }
        let beacon_block = BeaconBlockWithoutRoot::with_transactions(response.data.message, txs);
        Ok(beacon_block)
    }
}