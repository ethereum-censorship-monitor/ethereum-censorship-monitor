use std::collections::{HashMap, HashSet};

use ethers::types::Transaction;

use crate::db;
use crate::nonce_cache::NonceCache;
use crate::pool::{Pool, TransactionWithVisibility};
use crate::types::{BeaconBlock, TxHash};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Analysis {
    pub beacon_block: BeaconBlock<Transaction>,
    pub missing_transactions: HashMap<TxHash, TransactionWithVisibility>,
    pub included_transactions: HashMap<TxHash, TransactionWithVisibility>,
    pub num_txs_in_pool: usize,
    pub num_txs_in_block: usize,
    pub num_nonce_too_big: usize,
    pub num_only_tx_hash: usize,
    pub duration: Duration,
}

impl Analysis {
    pub fn summary(&self) -> String {
        format!(
            "Analysis for block {}: {} txs from pool included, {} missed, {} in pool, {} in block, {} nonce too big, {} only tx hash, took {}s",
            self.beacon_block,
            self.included_transactions.len(),
            self.missing_transactions.len(),
            self.num_txs_in_pool,
            self.num_txs_in_block,
            self.num_nonce_too_big,
            self.num_only_tx_hash,
            self.duration.as_secs(),
        )
    }
}

pub async fn analyze(
    beacon_block: &BeaconBlock<Transaction>,
    pool: &Pool,
    nonce_cache: &mut NonceCache,
) -> Analysis {
    let start_time = Instant::now();

    let exec = &beacon_block.body.execution_payload;
    let txs_in_block: HashSet<&TxHash> =
        HashSet::from_iter(exec.transactions.iter().map(|tx| &tx.hash));
    let mut missing_txs = HashMap::new();
    let mut included_txs = HashMap::new();

    let proposal_time = beacon_block.proposal_time();
    let pool_at_t = pool.content_at(proposal_time);
    let num_txs_in_pool = pool_at_t.len();
    let num_txs_in_block = exec.transactions.len();
    let mut num_nonce_too_big = 0;
    let mut num_only_tx_hash = 0;

    for (hash, tx_with_vis) in pool_at_t {
        if txs_in_block.contains(&hash) {
            included_txs.insert(hash, tx_with_vis);
        } else {
            if let Some(ref tx) = tx_with_vis.transaction {
                let nonce = nonce_cache
                    .get(&tx.from.unwrap(), beacon_block)
                    .await
                    .unwrap();
                if nonce == tx.nonce.as_u64() {
                    missing_txs.insert(hash, tx_with_vis);
                } else {
                    num_nonce_too_big += 1;
                }
            } else {
                num_only_tx_hash += 1;
            }
        }
    }

    let duration = start_time.elapsed();

    Analysis {
        beacon_block: beacon_block.clone(),
        missing_transactions: missing_txs,
        included_transactions: included_txs,
        num_txs_in_pool,
        num_txs_in_block,
        num_nonce_too_big,
        num_only_tx_hash,
        duration,
    }
}

// /// Return the timestamp at which the given block was supposed to be created.
// pub fn get_proposal_time<T>(block: &Block<T>) -> Timestamp {}

pub fn insert_analysis_into_db<T: db::DB>(analysis: &Analysis, db: &mut T) -> Result<(), T::Error> {
    // let block_hash = analysis.block.hash.unwrap();
    // for (hash, tx) in &analysis.missing_transactions {
    //     db.insert_tx(db::Tx { hash: *hash })?;
    //     db.insert_block(db::Block {
    //         hash: block_hash,
    //         proposer_index: 0,
    //     })?;
    //     db.insert_miss(db::Miss {
    //         tx: *hash,
    //         block: block_hash,
    //         delay: tx.first_seen - analysis.block.timestamp.as_u64(),
    //     })?;
    // }
    Ok(())
}
