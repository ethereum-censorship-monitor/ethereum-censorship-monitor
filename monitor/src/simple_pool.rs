use crate::types::{Timestamp, TxHash, TxpoolContent, TxpoolTransaction, H256};
use std::collections::{hash_map, HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct SeenTransaction {
    pub tx_hash: TxHash,
    pub first_seen: Timestamp,
    pub transaction: Option<TxpoolTransaction>,
}

#[derive(Debug)]
pub struct SimplePool {
    txs: HashMap<TxHash, SeenTransaction>,
    head_hash: H256,
}

impl SimplePool {
    pub fn new() -> SimplePool {
        SimplePool {
            txs: HashMap::new(),
            head_hash: H256::zero(),
        }
    }

    /// Inform the pool about the exact set of transactions it contains.
    pub fn update(&mut self, pool: &TxpoolContent, t: Timestamp) {
        let prev_tx_hashes: HashSet<TxHash> = self.txs.keys().copied().collect();

        let mut tx_hashes: HashSet<TxHash> = HashSet::new();
        for v in pool.pending.values().chain(pool.queued.values()) {
            for tx in v.values() {
                tx_hashes.insert(tx.hash);
                self.txs
                    .entry(tx.hash)
                    .and_modify(|st| {
                        st.first_seen = std::cmp::min(st.first_seen, t);
                        st.transaction = Some(tx.clone());
                    })
                    .or_insert_with(|| SeenTransaction {
                        tx_hash: tx.hash,
                        first_seen: t,
                        transaction: Some(tx.clone()),
                    });
            }
        }

        let removed_hashes = prev_tx_hashes.difference(&tx_hashes);
        for h in removed_hashes {
            self.txs.remove(h);
        }
    }

    /// Inform the pool about an individual transaction observed at the given timestamp.
    pub fn pre_announce_transaction(&mut self, hash: TxHash, t: Timestamp) {
        self.txs
            .entry(hash)
            .and_modify(|st| {
                st.first_seen = std::cmp::min(st.first_seen, t);
            })
            .or_insert_with(|| SeenTransaction {
                tx_hash: hash,
                first_seen: t,
                transaction: None,
            });
    }

    /// Set the hash of the head on which the transactions in the pool are meant to be applied.
    pub fn set_head_hash(&mut self, hash: H256) {
        self.head_hash = hash;
    }

    /// Create an iterator of all transactions that should be considered for building a block with
    /// the given parent hash.
    pub fn iter_candidates(
        &self,
        parent_hash: H256,
    ) -> Option<hash_map::Iter<TxHash, SeenTransaction>> {
        if parent_hash == self.head_hash {
            Some(self.txs.iter())
        } else {
            None
        }
    }
}
