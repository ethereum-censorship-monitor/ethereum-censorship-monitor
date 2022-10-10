use std::collections::{HashMap, HashSet};

use crate::db;
use crate::simple_pool::{SeenTransaction, SimplePool};
use crate::types::{Block, TxHash, H256};

#[derive(Debug)]
pub struct Analysis {
    pub block: Block<H256>,
    pub missing_transactions: HashMap<TxHash, SeenTransaction>,
    pub included_transactions: HashMap<TxHash, SeenTransaction>,
}

pub fn analyze(block: &Block<H256>, pool: &SimplePool) -> Analysis {
    let txs_in_block: HashSet<&TxHash> = HashSet::from_iter(&block.transactions);
    let mut missing_txs = HashMap::new();
    let mut included_txs = HashMap::new();
    if let Some(iter) = pool.iter_candidates(block.parent_hash) {
        for (hash, tx) in iter {
            if txs_in_block.contains(&hash) {
                included_txs.insert(*hash, tx.clone());
            } else {
                missing_txs.insert(*hash, tx.clone());
            }
        }
    }
    Analysis {
        block: block.clone(),
        missing_transactions: missing_txs,
        included_transactions: included_txs,
    }
}

pub fn insert_analysis_into_db<T: db::DB>(analysis: &Analysis, db: &mut T) -> Result<(), T::Error> {
    let block_hash = analysis.block.hash.unwrap();
    for (hash, tx) in &analysis.missing_transactions {
        db.insert_tx(db::Tx { hash: *hash })?;
        db.insert_block(db::Block {
            hash: block_hash,
            proposer_index: 0,
        })?;
        db.insert_miss(db::Miss {
            tx: *hash,
            block: block_hash,
            delay: tx.first_seen - analysis.block.timestamp.as_u64(),
        })?;
    }
    Ok(())
}
