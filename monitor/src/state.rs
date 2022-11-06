use std::time::Instant;

use crate::analyze::{analyze, Analysis};
use crate::head_history::HeadHistory;
use crate::nonce_cache::{NonceCache, NonceCacheError};
use crate::pool::Pool;
use crate::types::{BeaconBlock, Timestamp, TxHash, TxpoolContent, H256};
use crate::watch::{Event, NodeConfig};
use ethers::providers::ProviderError;
use ethers::types::Transaction;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("{0}")]
    NonceCacheError(NonceCacheError),
    #[error("{0}")]
    ProviderError(ProviderError),
}

pub struct State {
    pool: Pool,
    head_history: HeadHistory,
    nonce_cache: NonceCache,

    num_received_blocks: usize,
}

impl State {
    pub fn new(node_config: &NodeConfig) -> Self {
        let pool = Pool::new();
        let head_history = HeadHistory::new();

        let nonce_cache_provider = node_config.http_provider();
        let nonce_cache = NonceCache::new(nonce_cache_provider);

        State {
            pool,
            head_history,
            nonce_cache,

            num_received_blocks: 0,
        }
    }

    pub async fn process_event(&mut self, event: Event) -> Option<Analysis> {
        match event {
            Event::NewTransaction { hash, timestamp } => {
                self.process_new_transaction_event(hash, timestamp).await
            }
            Event::NewHead {
                beacon_block,
                timestamp,
            } => {
                let start_time = Instant::now();
                let r = self.process_new_head_event(beacon_block, timestamp).await;
                let duration = start_time.elapsed();
                log::info!("analysis took {}s", duration.as_secs());
                r
            }
            Event::TxpoolContent { content, timestamp } => {
                self.process_txpool_content_event(content, timestamp).await
            }
        }
    }

    async fn process_new_transaction_event(
        &mut self,
        hash: TxHash,
        t: Timestamp,
    ) -> Option<Analysis> {
        self.pool.pre_announce_transaction(t, hash);
        None
    }

    async fn process_txpool_content_event(
        &mut self,
        content: TxpoolContent,
        t: Timestamp,
    ) -> Option<Analysis> {
        self.pool.observe(t, content);
        None
    }

    async fn process_new_head_event(
        &mut self,
        beacon_block: BeaconBlock<Transaction>,
        t: Timestamp,
    ) -> Option<Analysis> {
        self.num_received_blocks += 1;
        log::info!("processing block {}", beacon_block);
        self.head_history.observe(t, beacon_block.clone());
        self.nonce_cache.apply_block(beacon_block.clone());

        if self.num_received_blocks <= 1 {
            // don't try to analyze because we know we don't have the parent block
            return None;
        }
        let parent = self.head_history.at(beacon_block.proposal_time());
        if match parent {
            None => {
                log::info!("skipping analysis, no idea if our view of parent block matched");
                true
            }
            Some(parent_observation) => {
                if parent_observation.head.root != beacon_block.parent_root {
                    log::info!("skipping analysis, head mismatch at proposal time");
                    true
                } else {
                    false
                }
            }
        } {
            return None;
        }

        log::info!(
            "analyzing block {} with {} transactions",
            beacon_block,
            &beacon_block.body.execution_payload.transactions.len()
        );
        Some(analyze(&beacon_block, &self.pool, &mut self.nonce_cache).await)
    }
}
