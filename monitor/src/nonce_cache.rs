use crate::types::{Address, BeaconBlock, H256};
use ethers::providers::{Http, Middleware, Provider, ProviderError};
use ethers::types::{BlockId, Transaction};
use std::collections::HashMap;
use thiserror::Error;

pub struct NonceCache {
    beacon_block: BeaconBlock<Transaction>,
    nonces: HashMap<Address, u64>,
    provider: Provider<Http>,
}

#[derive(Debug, Error)]
pub enum NonceCacheError {
    #[error("failed to fetch nonce")]
    ProviderError(#[from] ProviderError),
    #[error("nonce cache is at block hash {internal}, but was queried at {queried}")]
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

        let mut num_modified = 0;
        for tx in &self.beacon_block.body.execution_payload.transactions {
            self.nonces.entry(tx.from).and_modify(|n| {
                *n = tx.nonce.as_u64() + 1;
                num_modified += 1;
            });
        }
        log::debug!(
            "applied block {} to nonce cache, updating {} of {} entries",
            self.beacon_block,
            num_modified,
            self.nonces.len(),
        );
    }
}
