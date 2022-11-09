use ethers::types::TxpoolContent;
use std::collections::{HashMap, HashSet};

use crate::types::{Timestamp, Transaction, TxHash};
use crate::visibility::{Observation, Observations, Visibility};

#[derive(Debug)]
pub struct TransactionWithVisibility {
    pub hash: TxHash,
    pub transaction: Option<Transaction>,
    pub visibility: Visibility,
}

/// This struct keeps track of the current tx pool and how it changes over time. It provides
/// a method to query the set of transactions and their visibilities that were present at a given
/// point in time.
pub struct Pool {
    last_content: HashSet<TxHash>,
    tx_obs: HashMap<TxHash, Observations>,
    txs: HashMap<TxHash, Transaction>,
}

impl Pool {
    pub fn new() -> Self {
        Pool {
            last_content: HashSet::new(),
            tx_obs: HashMap::new(),
            txs: HashMap::new(),
        }
    }

    /// Inform the pool about the exact set of transactions it should contain. The transactions
    /// will be observed as Seen. The transactions that are missing compared to the previous
    /// invocation (or intermediate additions via pre_announce_transaction) are observed as
    /// NotSeen.
    pub fn observe(&mut self, timestamp: Timestamp, content: TxpoolContent) {
        let mut num_new = 0;
        let mut num_backfills = 0;

        let mut txs: HashMap<TxHash, &Transaction> = HashMap::new();
        for v in content.pending.values().chain(content.queued.values()) {
            for tx in v.values() {
                let r = txs.insert(tx.hash, tx);
                if r.is_none() {
                    if self.tx_obs.contains_key(&tx.hash) {
                        num_backfills += 1;
                    } else {
                        num_new += 1;
                    }
                }
            }
        }
        let hashes: HashSet<TxHash> = txs.keys().copied().collect();

        let unseen_hashes = self.last_content.difference(&hashes);
        let unseen_hashes: HashSet<TxHash> = unseen_hashes.copied().collect();
        self.last_content = hashes;

        // insert txs from pool as Seen
        for hash in &self.last_content {
            self.tx_obs
                .entry(hash.clone())
                .or_insert_with(Observations::new)
                .insert(Observation::Seen(timestamp));
            self.txs.insert(*hash, (*txs.get(hash).unwrap()).clone());
        }

        // insert txs not in pool anymore as NotSeen
        for hash in &unseen_hashes {
            if let Some(obs) = self.tx_obs.get_mut(&hash) {
                obs.insert(Observation::NotSeen(timestamp));
            }
        }

        log::debug!(
            "observed pool at timestamp {} with {} txs ({} new, {} backfills, {} unseen, {} total entries, {} only hashes)",
            timestamp,
            self.last_content.len(),
            num_new,
            num_backfills,
            unseen_hashes.len(),
            self.tx_obs.len(),
            self.tx_obs.len() - self.txs.len(),
        );
    }

    /// Inform the pool about an individual transaction observed as Seen at the given timestamp.
    pub fn pre_announce_transaction(&mut self, t: Timestamp, hash: TxHash) {
        self.tx_obs
            .entry(hash.clone())
            .or_insert_with(Observations::new)
            .insert(Observation::Seen(t));
    }

    /// Query the set of transactions that are either visible or disappearing at the given
    /// timestamp.
    pub fn content_at(&self, t: Timestamp) -> HashMap<TxHash, TransactionWithVisibility> {
        let mut txs: HashMap<TxHash, TransactionWithVisibility> = HashMap::new();
        for (tx_hash, obs) in &self.tx_obs {
            let vis = obs.visibility_at(t);
            if match vis {
                Visibility::Visible {
                    first_seen: _,
                    last_seen: _,
                } => true,
                Visibility::Disappearing {
                    first_seen: _,
                    last_seen: _,
                    disappeared: _,
                } => true,
                Visibility::Invisible { disappeared: _ } => false,
            } {
                let tx = self.txs.get(tx_hash).map(|tx| tx.clone());
                let tx_with_vis = TransactionWithVisibility {
                    hash: *tx_hash,
                    transaction: tx,
                    visibility: vis,
                };
                txs.insert(tx_hash.clone(), tx_with_vis);
            }
        }
        txs
    }

    /// Prune deletes all data that does not affect visibilities at or after the given timestamp.
    pub fn prune(&mut self, cutoff: Timestamp) {
        let mut fully_pruned: HashSet<TxHash> = HashSet::new();
        for (tx_hash, obs) in self.tx_obs.iter_mut() {
            obs.prune(cutoff);
            if obs.is_empty() {
                fully_pruned.insert(*tx_hash);
            }
        }
        for tx_hash in &fully_pruned {
            self.tx_obs.remove(&tx_hash);
            self.txs.remove(&tx_hash);
        }
        log::debug!(
            "pruned {} txs from pool before time {} to a new size of {}",
            fully_pruned.len(),
            cutoff,
            self.tx_obs.len()
        );
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use ethers::types::Address;

    use super::*;

    const H1: TxHash = TxHash::repeat_byte(1);
    const H2: TxHash = TxHash::repeat_byte(2);

    fn make_pool(hashes: Vec<TxHash>) -> TxpoolContent {
        let queued = BTreeMap::new();

        let mut pending = BTreeMap::new();
        let mut txs = BTreeMap::new();
        let address = Address::repeat_byte(0);
        for (i, h) in hashes.iter().enumerate() {
            let mut tx = Transaction::default();
            tx.hash = *h;
            txs.insert(i.to_string(), tx);
        }
        pending.insert(address, txs);

        TxpoolContent { pending, queued }
    }

    fn assert_content(c: HashMap<TxHash, TransactionWithVisibility>, v: Vec<(TxHash, Visibility)>) {
        assert_eq!(c.len(), v.len());
        for (h, vis) in v {
            let tx_with_vis = c.get(&h);
            let tx_with_vis = tx_with_vis.unwrap();
            assert_eq!(tx_with_vis.visibility, vis);
            assert_eq!(tx_with_vis.hash, h);
        }
    }

    #[test]
    fn test_observe() {
        let mut p = Pool::new();
        p.observe(10, make_pool(vec![H1, H2]));
        p.observe(20, make_pool(vec![H1]));

        assert_content(p.content_at(9), vec![].into_iter().collect());
        assert_content(
            p.content_at(10),
            vec![
                (
                    H1,
                    Visibility::Visible {
                        first_seen: 10,
                        last_seen: 20,
                    },
                ),
                (
                    H2,
                    Visibility::Disappearing {
                        first_seen: 10,
                        last_seen: 10,
                        disappeared: 20,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        );
        assert_content(
            p.content_at(20),
            vec![(
                H1,
                Visibility::Visible {
                    first_seen: 10,
                    last_seen: 20,
                },
            )]
            .into_iter()
            .collect(),
        );
    }

    #[test]
    fn test_pre_announce() {
        let mut p = Pool::new();
        p.pre_announce_transaction(10, H1);
        assert_content(
            p.content_at(10),
            vec![(
                H1,
                Visibility::Visible {
                    first_seen: 10,
                    last_seen: 10,
                },
            )]
            .into_iter()
            .collect(),
        );
    }

    #[test]
    fn test_prune() {
        let mut p = Pool::new();
        p.observe(10, make_pool(vec![H1, H2]));
        p.observe(20, make_pool(vec![H1]));
        p.prune(20);
        assert_eq!(p.content_at(19).len(), 0);
        assert_content(
            p.content_at(20),
            vec![(
                H1,
                Visibility::Visible {
                    first_seen: 20,
                    last_seen: 20,
                },
            )]
            .into_iter()
            .collect(),
        );
    }
}
