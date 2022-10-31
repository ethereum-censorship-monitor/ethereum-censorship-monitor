use crate::types::{Address, Block, MissingBlockFieldError, Transaction, H256, U256};
use ethers::{
    providers::{Http, Middleware, Provider, ProviderError},
    types::BlockId,
};
use std::collections::HashMap;
use thiserror::Error;

pub struct NonceCache {
    head_hash: H256,
    nonces: HashMap<Address, u64>,
    provider: Provider<Http>,
}

#[derive(Debug, Error)]
pub enum NonceCacheError {
    #[error("provider error: {0}")]
    ProviderError(ProviderError),
    #[error("failed to fetch block {n} as it does not exist")]
    MissingBlockError { n: u64 },
    #[error("{0}")]
    MissingBlockFieldError(MissingBlockFieldError),
}

impl NonceCache {
    pub fn new(provider: Provider<Http>, head_hash: H256) -> Self {
        NonceCache {
            head_hash,
            nonces: HashMap::new(),
            provider,
        }
    }

    pub async fn initialize(
        provider: Provider<Http>,
        delay: usize,
    ) -> Result<Self, NonceCacheError> {
        let initial_block_number = provider
            .get_block_number()
            .await
            .map_err(NonceCacheError::ProviderError)?;
        let initial_head = provider
            .get_block(initial_block_number - delay)
            .await
            .map_err(NonceCacheError::ProviderError)?
            .ok_or(NonceCacheError::MissingBlockError {
                n: initial_block_number.as_u64(),
            })?;
        let initial_hash = initial_head
            .hash
            .ok_or(NonceCacheError::MissingBlockFieldError(
                MissingBlockFieldError::new(String::from("hash")),
            ))?;
        Ok(Self::new(provider, initial_hash))
    }

    pub async fn get(&mut self, account: &Address) -> Result<u64, ProviderError> {
        match self.nonces.get(account) {
            Some(&n) => Ok(n),
            None => {
                let nonce_u256 = self
                    .provider
                    .get_transaction_count(account.clone(), Some(BlockId::Hash(self.head_hash)))
                    .await?;
                let nonce = nonce_u256.as_u64();
                self.nonces.insert(account.clone(), nonce);
                Ok(nonce)
            }
        }
    }

    pub fn apply_head(&mut self, head: &Block<Transaction>) {
        if self.head_hash != head.parent_hash {
            self.nonces.clear();
        }
        self.head_hash = head.hash.unwrap();
        for tx in &head.transactions {
            self.nonces
                .entry(tx.from)
                .and_modify(|n| *n = tx.nonce.as_u64() + 1);
        }
    }
}
