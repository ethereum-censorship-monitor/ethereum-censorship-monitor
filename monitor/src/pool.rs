use std::{cmp::min, collections::HashMap};

use chrono::{DateTime, Utc};
use ethers::types::TxpoolContent;

use crate::types::{NodeKey, Transaction, TxHash};

/// ObservedTransaction stores a transaction hash and optionally a transaction
/// body along with information about its observation history. For each node it
/// stores the timestamp at which they have first observed the transaction. In
/// addition, it stores the timestamp at which the transactions was first
/// observed to have disappeared from the pool on the main node.
#[derive(Debug, Clone)]
pub struct ObservedTransaction {
    pub hash: TxHash,
    pub transaction: Option<Transaction>,
    pub first_seen: HashMap<NodeKey, DateTime<Utc>>,
    pub disappeared: Option<DateTime<Utc>>,
}

impl ObservedTransaction {
    /// Create a new transaction identified by its hash.
    pub fn new(hash: TxHash) -> Self {
        ObservedTransaction {
            hash,
            transaction: None,
            first_seen: HashMap::new(),
            disappeared: None,
        }
    }

    /// Set the first seen time at the given node. If a timestamp has already
    /// been recorded for the node, keep the earlier one.
    pub fn observe(&mut self, node_key: NodeKey, timestamp: DateTime<Utc>) {
        let t = self.first_seen.entry(node_key).or_insert(timestamp);
        *t = min(*t, timestamp);
    }

    /// Set the disappeared timestamp. If a timestamp has already been recorded,
    /// keep the earlier one.
    pub fn disappear_at(&mut self, timestamp: DateTime<Utc>) {
        let t = self.disappeared.get_or_insert(timestamp);
        *t = min(*t, timestamp);
    }

    /// Delete both first seen and disappearance timestamps.
    pub fn clear_observations(&mut self) {
        self.first_seen.clear();
        self.disappeared = None;
    }

    /// Count the number of nodes that have seen the transactions at or before
    /// the given timestamps.
    pub fn num_nodes_seen(&self, timestamp: DateTime<Utc>) -> usize {
        self.first_seen
            .values()
            .filter(|&&t| t <= timestamp)
            .count()
    }

    /// Return the earliest timestamp at which a given number of nodes have seen
    /// the transaction.
    pub fn quorum_reached_timestamp(&self, quorum: usize) -> Option<DateTime<Utc>> {
        let mut timestamps: Vec<&DateTime<Utc>> = self.first_seen.values().collect();
        if timestamps.len() < quorum {
            return None;
        }
        timestamps.sort();
        Some(*timestamps[quorum - 1])
    }

    /// Check if the transaction has disappeared at or before the given
    /// timestamp.
    pub fn has_disappeared_before(&self, timestamp: DateTime<Utc>) -> bool {
        self.disappeared.map_or(false, |t| timestamp >= t)
    }
}

/// This struct keeps track of transactions we observed in the network.
#[derive(Debug)]
pub struct Pool(HashMap<TxHash, ObservedTransaction>);

impl Pool {
    /// Create a new empty pool.
    pub fn new() -> Self {
        Pool(HashMap::new())
    }

    /// Get the transactions that have been observed at least once at or before
    /// the given timestamp and have not disappeared yet.
    pub fn content_at(&self, timestamp: DateTime<Utc>) -> HashMap<TxHash, ObservedTransaction> {
        self.0
            .values()
            .filter(|tx| tx.num_nodes_seen(timestamp) >= 1 && !tx.has_disappeared_before(timestamp))
            .map(|tx| (tx.hash, tx.clone()))
            .collect()
    }

    /// Insert a transaction into the pool observed on the given node at the
    /// given time.
    pub fn observe_transaction(
        &mut self,
        node_key: NodeKey,
        timestamp: DateTime<Utc>,
        hash: TxHash,
    ) {
        self.0
            .entry(hash)
            .or_insert_with(|| ObservedTransaction::new(hash))
            .observe(node_key, timestamp);
    }

    /// Update the pool with a full snapshot of transactions in it taken on the
    /// given node at a given time. This not only inserts the transactions,
    /// but also marks transactions that are missing as disappeared.
    pub fn observe_pool(
        &mut self,
        node_key: NodeKey,
        timestamp: DateTime<Utc>,
        content: TxpoolContent,
    ) {
        let txs: HashMap<TxHash, &Transaction> = content
            .pending
            .values()
            .chain(content.queued.values())
            .flat_map(|m| m.values())
            .map(|tx| (tx.hash, tx))
            .collect();
        let num_txs = txs.len();

        // observe txs in pool
        let mut num_new = 0;
        let mut num_new_objects = 0;
        let mut num_reappeared = 0;
        for (tx_hash, &tx) in &txs {
            let obs_tx = self.0.entry(*tx_hash).or_insert_with(|| {
                num_new += 1;
                ObservedTransaction::new(*tx_hash)
            });
            if obs_tx.transaction.is_none() {
                num_new_objects += 1;
                obs_tx.transaction = Some(tx.clone());
            }
            if obs_tx.has_disappeared_before(timestamp) {
                num_reappeared += 1;
                obs_tx.clear_observations();
            }
            obs_tx.observe(node_key, timestamp);
        }
        let num_backfills = num_new_objects - num_new;

        // mark transactions not in the pool as disappeared
        let mut num_disappeared = 0;
        for (tx_hash, obs_tx) in self.0.iter_mut() {
            if !txs.contains_key(tx_hash) {
                if !obs_tx.has_disappeared_before(timestamp) {
                    num_disappeared += 1;
                }
                obs_tx.disappear_at(timestamp);
            }
        }

        log::debug!(
            "observed pool with {} txs ({} new, {} backfills, {} disappeared,
{} reappeared, total size {})",
            num_txs,
            num_new,
            num_backfills,
            num_disappeared,
            num_reappeared,
            self.0.len(),
        );
    }

    /// Remove transactions that have already disappeared at the given
    /// timestamp.
    #[allow(dead_code)]
    pub fn prune(&mut self, cutoff: DateTime<Utc>) {
        let len_before = self.0.len();
        self.0
            .retain(|_, obs_tx| !obs_tx.has_disappeared_before(cutoff));
        let len_after = self.0.len();
        log::debug!(
            "pruned pool from {} to {} by {} transactions",
            len_before,
            len_after,
            len_before - len_after
        );
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use chrono::TimeZone;
    use ethers::types::Address;

    use super::*;

    const H1: TxHash = TxHash::repeat_byte(1);
    const H2: TxHash = TxHash::repeat_byte(2);

    fn t(s: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(s, 0).unwrap()
    }

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

    fn assert_content(
        c: HashMap<TxHash, ObservedTransaction>,
        v: Vec<(TxHash, bool, Vec<DateTime<Utc>>, Option<DateTime<Utc>>)>,
    ) {
        assert_eq!(c.len(), v.len());
        for (h, has_body, first_seen, disappeared) in v {
            let obs_tx = c.get(&h).unwrap();
            assert_eq!(obs_tx.hash, h);
            if has_body {
                assert_eq!(obs_tx.transaction.clone().unwrap().hash, h);
            } else {
                assert!(obs_tx.transaction.is_none());
            }
            assert_eq!(obs_tx.first_seen.len(), first_seen.len());
            for (i, t) in first_seen.iter().enumerate() {
                assert_eq!(obs_tx.first_seen.get(&i).unwrap(), t);
            }
            assert_eq!(obs_tx.disappeared, disappeared);
        }
    }

    #[test]
    fn test_tx_new() {
        let obs_tx = ObservedTransaction::new(H1);
        assert_eq!(obs_tx.hash, H1);
        assert!(obs_tx.transaction.is_none());
        assert!(obs_tx.first_seen.is_empty());
        assert!(obs_tx.disappeared.is_none());
    }

    #[test]
    fn test_tx_observe() {
        let mut obs_tx = ObservedTransaction::new(H1);
        assert_eq!(obs_tx.first_seen.len(), 0);
        assert_eq!(obs_tx.num_nodes_seen(t(0)), 0);

        obs_tx.observe(0, t(20));
        assert_eq!(obs_tx.first_seen.len(), 1);
        assert_eq!(*obs_tx.first_seen.get(&0).unwrap(), t(20));
        assert_eq!(obs_tx.num_nodes_seen(t(19)), 0);
        assert_eq!(obs_tx.num_nodes_seen(t(20)), 1);

        obs_tx.observe(0, t(25));
        assert_eq!(obs_tx.first_seen.len(), 1);
        assert_eq!(*obs_tx.first_seen.get(&0).unwrap(), t(20));

        obs_tx.observe(0, t(15));
        assert_eq!(obs_tx.first_seen.len(), 1);
        assert_eq!(*obs_tx.first_seen.get(&0).unwrap(), t(15));
        assert_eq!(obs_tx.num_nodes_seen(t(14)), 0);
        assert_eq!(obs_tx.num_nodes_seen(t(15)), 1);

        obs_tx.observe(1, t(20));
        assert_eq!(obs_tx.first_seen.len(), 2);
        assert_eq!(*obs_tx.first_seen.get(&0).unwrap(), t(15));
        assert_eq!(*obs_tx.first_seen.get(&1).unwrap(), t(20));
        assert_eq!(obs_tx.num_nodes_seen(t(19)), 1);
        assert_eq!(obs_tx.num_nodes_seen(t(20)), 2);
    }

    #[test]
    fn test_tx_disappear() {
        let mut obs_tx = ObservedTransaction::new(H1);
        assert!(obs_tx.disappeared.is_none());
        assert!(!obs_tx.has_disappeared_before(t(100)));

        obs_tx.disappear_at(t(20));
        assert_eq!(obs_tx.disappeared, Some(t(20)));
        assert!(!obs_tx.has_disappeared_before(t(19)));
        assert!(obs_tx.has_disappeared_before(t(20)));

        obs_tx.disappear_at(t(25));
        assert_eq!(obs_tx.disappeared, Some(t(20)));

        obs_tx.disappear_at(t(15));
        assert_eq!(obs_tx.disappeared, Some(t(15)));
        assert!(!obs_tx.has_disappeared_before(t(14)));
        assert!(obs_tx.has_disappeared_before(t(15)));
    }

    #[test]
    fn test_tx_clear() {
        let mut obs_tx = ObservedTransaction::new(H1);
        obs_tx.observe(0, t(10));
        obs_tx.disappear_at(t(20));
        obs_tx.clear_observations();
        assert!(obs_tx.first_seen.is_empty());
        assert!(obs_tx.disappeared.is_none());
    }

    #[test]
    fn test_observe_pool() {
        let mut p = Pool::new();
        p.observe_pool(0, t(10), make_pool(vec![H1, H2]));
        p.observe_pool(0, t(20), make_pool(vec![H1]));

        assert_content(p.content_at(t(9)), vec![].into_iter().collect());
        assert_content(
            p.content_at(t(10)),
            vec![
                (H1, true, vec![t(10)], None),
                (H2, true, vec![t(10)], Some(t(20))),
            ]
            .into_iter()
            .collect(),
        );
        assert_content(
            p.content_at(t(20)),
            vec![(H1, true, vec![t(10)], None)].into_iter().collect(),
        );
    }

    #[test]
    fn test_observe_transaction() {
        let mut p = Pool::new();
        p.observe_transaction(0, t(10), H1);
        p.observe_transaction(1, t(11), H1);
        assert_content(
            p.content_at(t(10)),
            vec![(H1, false, vec![t(10), t(11)], None)]
                .into_iter()
                .collect(),
        );
    }

    #[test]
    fn test_backfill() {
        let mut p = Pool::new();
        p.observe_transaction(0, t(10), H1);
        p.observe_pool(0, t(20), make_pool(vec![H1]));
        assert_content(
            p.content_at(t(10)),
            vec![(H1, true, vec![t(10)], None)].into_iter().collect(),
        );
    }

    #[test]
    fn test_prune() {
        let mut p = Pool::new();
        p.observe_pool(0, t(10), make_pool(vec![H1, H2]));
        p.observe_pool(0, t(20), make_pool(vec![H1]));
        p.observe_pool(0, t(30), make_pool(vec![]));
        assert_eq!(p.content_at(t(10)).len(), 2);

        p.prune(t(19));
        assert_eq!(p.content_at(t(10)).len(), 2);

        p.prune(t(20));
        assert_eq!(p.content_at(t(10)).len(), 1);
        assert_content(
            p.content_at(t(10)),
            vec![(H1, true, vec![t(10)], Some(t(30)))]
                .into_iter()
                .collect(),
        );
    }
}
