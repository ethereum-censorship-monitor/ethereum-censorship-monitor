use std::{
    collections::{BTreeMap, HashMap},
    time::Instant,
};

use ethers::{
    providers::{Http, Middleware, Provider, ProviderError},
    types::{BlockId, Transaction},
};
use thiserror::Error;

use crate::{
    metrics,
    types::{Address, BeaconBlock, H256},
};

pub struct NonceCache {
    beacon_block: BeaconBlock<Transaction>,
    nonces: HashMap<Address, u64>,
    last_access_time: BTreeMap<Address, Instant>,
    max_size: usize,
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
    pub fn new(provider: Provider<Http>, max_size: usize) -> Self {
        let c = NonceCache {
            beacon_block: BeaconBlock::default(),
            nonces: HashMap::new(),
            last_access_time: BTreeMap::new(),
            max_size,
            provider,
        };
        c.report();
        c
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

        self.last_access_time.insert(*account, Instant::now());

        let block_id = Some(BlockId::Hash(
            beacon_block.body.execution_payload.block_hash,
        ));
        match self.nonces.get(account) {
            Some(&n) => Ok(n),
            None => {
                let nonce_u256 = self
                    .provider
                    .get_transaction_count(*account, block_id)
                    .await
                    .map_err(NonceCacheError::ProviderError)?;
                let nonce = nonce_u256.as_u64();
                self.nonces.insert(*account, nonce);
                self.prune();
                self.report();
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
        self.report();
        log::debug!(
            "applied block {} to nonce cache, updating {} of {} entries",
            self.beacon_block,
            num_modified,
            self.nonces.len(),
        );
    }

    fn prune(&mut self) {
        while self.nonces.len() > self.max_size {
            if let Some(oldest_account) = self.last_access_time.pop_first() {
                self.nonces.remove(&oldest_account.0);
            } else {
                log::error!(
                    "failed to prune nonce cache: last access time map is empty, but still too \
                     many nonces"
                );
            }
        }
    }

    fn report(&self) {
        metrics::NONCE_CACHE_SIZE.set(self.nonces.len() as i64);
    }
}
