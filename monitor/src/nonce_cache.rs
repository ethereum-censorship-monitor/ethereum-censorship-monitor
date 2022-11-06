use crate::types::{Address, BeaconBlock, H256};
use ethers::{
    providers::{Http, Middleware, Provider, ProviderError},
    types::{BlockId, Transaction},
};
use std::collections::HashMap;
use thiserror::Error;

pub struct NonceCache {
    beacon_block: BeaconBlock<Transaction>,
    nonces: HashMap<Address, u64>,
    provider: Provider<Http>,
}

#[derive(Debug, Error)]
pub enum NonceCacheError {
    #[error("provider error: {0}")]
    ProviderError(ProviderError),
    #[error(
        "failed to fetch block {:?} as it does not exist",
        .n.map(|n| n.to_string()).or(.hash.map(|h| h.to_string())).unwrap())]
    MissingBlockError { n: Option<u64>, hash: Option<H256> },
    #[error("queried cache at block hash {queried} instead of {internal}")]
    WrongBlockError { internal: H256, queried: H256 },
}

impl NonceCache {
    pub fn new(provider: Provider<Http>) -> Self {
        NonceCache {
            beacon_block: BeaconBlock::default(),
            nonces: HashMap::new(),
            provider,
        }
    }

    pub async fn get(
        &mut self,
        account: &Address,
        beacon_block: &BeaconBlock<Transaction>,
    ) -> Result<u64, NonceCacheError> {
        if beacon_block.root != self.beacon_block.root {
            return Err(NonceCacheError::WrongBlockError {
                internal: self.beacon_block.root,
                queried: beacon_block.root,
            });
        }

        let block_id = Some(BlockId::Hash(
            beacon_block.body.execution_payload.block_hash,
        ));
        match self.nonces.get(account) {
            Some(&n) => Ok(n),
            None => {
                let nonce_u256 = self
                    .provider
                    .get_transaction_count(account.clone(), block_id)
                    .await
                    .map_err(NonceCacheError::ProviderError)?;
                let nonce = nonce_u256.as_u64();
                self.nonces.insert(account.clone(), nonce);
                Ok(nonce)
            }
        }
    }

    pub fn apply_block(&mut self, beacon_block: BeaconBlock<Transaction>) {
        if beacon_block.parent_root != self.beacon_block.root {
            log::info!(
                "clearing nonce cache due to reorg from {} to {}",
                self.beacon_block,
                beacon_block,
            );
            self.nonces.clear();
        }
        self.beacon_block = beacon_block;

        for tx in &self.beacon_block.body.execution_payload.transactions {
            self.nonces
                .entry(tx.from)
                .and_modify(|n| *n = tx.nonce.as_u64() + 1);
        }
    }
}
