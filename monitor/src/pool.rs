use std::{
    cmp::{max, min},
    collections::HashMap,
};

use ethers::types::TxpoolContent;

use crate::types::{Timestamp, Transaction, TxHash};

/// ObservedTransaction stores a transaction hash and optionally a transaction
/// body along with information about its observation history. Three timestamps
/// make up the history: The first two define an interval in which the
/// transaction was visible and the last one the (optional) time at which is was
/// first observed to have disappeared.
#[derive(Debug, Clone)]
pub struct ObservedTransaction {
    pub hash: TxHash,
    pub transaction: Option<Transaction>,
    pub interval: (Timestamp, Timestamp),
    pub disappeared: Option<Timestamp>,
}

impl ObservedTransaction {
    /// Create a new transaction with a hash observed at a given timestamp.
    pub fn new(hash: TxHash, timestamp: Timestamp) -> Self {
        ObservedTransaction {
            hash,
            transaction: None,
            interval: (timestamp, timestamp),
            disappeared: None,
        }
    }

    /// Update the history with an observation at which the transaction was
    /// seen. The observation interval will be extended to include the new
    /// timestamp. In the case that the transaction has already disappeared
    /// at this time, the observation history will be reset completely. This is
    /// because reappearing transaction cannot be represented.
    pub fn observe_at(&mut self, timestamp: Timestamp) {
        if !self.has_disappeared_at(timestamp) {
            self.interval = (
                min(self.interval.0, timestamp),
                max(self.interval.1, timestamp),
            );
        } else {
            self.interval = (timestamp, timestamp);
            self.disappeared = None;
        }
    }

    /// Update the history with an observation at which the transaction was not
    /// seen. This sets the disappeared timestamp if it is lower than the
    /// existing one. In the case that the given timestamp falls into the
    /// seen interval, the interval will be clamped in order to ensure there
    /// is no overlap between the seen interval and the disappeared period.
    pub fn disappear_at(&mut self, timestamp: Timestamp) {
        let t = self.disappeared.get_or_insert(Timestamp::MAX);
        *t = min(*t, timestamp);
        self.interval = (
            min(self.interval.0, timestamp),
            min(self.interval.1, timestamp),
        );
    }

    /// Check if the transaction is visible at a given timestamp.
    pub fn is_visible_at(&self, timestamp: Timestamp) -> bool {
        timestamp >= self.interval.0 && !self.has_disappeared_at(timestamp)
    }

    /// Check if the transaction has disappeared already at a given timestamp.
    pub fn has_disappeared_at(&self, timestamp: Timestamp) -> bool {
        self.disappeared.map_or(false, |t| timestamp >= t)
    }
}

/// The pool keeps track of the elements we observe in a node's transaction
/// pool.
#[derive(Debug)]
pub struct Pool(HashMap<TxHash, ObservedTransaction>);

impl Pool {
    /// Create a new empty pool.
    pub fn new() -> Self {
        Pool(HashMap::new())
    }

    /// Get the transactions that are visible at the given timestamp.
    pub fn content_at(&self, timestamp: Timestamp) -> HashMap<TxHash, ObservedTransaction> {
        self.0
            .values()
            .filter(|tx| tx.is_visible_at(timestamp))
            .map(|tx| (tx.hash, tx.clone()))
            .collect()
    }

    /// Insert a transaction into the pool only knowing the hash and observation
    /// timestamp.
    pub fn pre_announce_transaction(&mut self, timestamp: Timestamp, hash: TxHash) {
        self.0
            .entry(hash)
            .or_insert_with(|| ObservedTransaction::new(hash, timestamp))
            .observe_at(timestamp);
    }

    /// Update the pool with a full snapshot of transactions in it taken at a
    /// given time. This not only inserts the transactions, it also marks
    /// transactions that are missing as disappeared.
    pub fn observe(&mut self, timestamp: Timestamp, content: TxpoolContent) {
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
                ObservedTransaction::new(*tx_hash, timestamp)
            });
            if obs_tx.transaction.is_none() {
                num_new_objects += 1;
                obs_tx.transaction = Some(tx.clone());
            }
            if obs_tx.has_disappeared_at(timestamp) {
                num_reappeared += 1;
            }
            obs_tx.observe_at(timestamp);
        }
        let num_backfills = num_new_objects - num_new;

        // mark transactions not in the pool as disappeared
        let mut num_disappeared = 0;
        for (tx_hash, obs_tx) in self.0.iter_mut() {
            if !txs.contains_key(tx_hash) {
                if !obs_tx.has_disappeared_at(timestamp) {
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
    pub fn prune(&mut self, cutoff: Timestamp) {
        let len_before = self.0.len();
        self.0
            .retain(|_, obs_tx| !obs_tx.has_disappeared_at(cutoff));
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

    fn assert_content(
        c: HashMap<TxHash, ObservedTransaction>,
        v: Vec<(TxHash, bool, (Timestamp, Timestamp), Option<Timestamp>)>,
    ) {
        assert_eq!(c.len(), v.len());
        for (h, has_body, interval, disappeared) in v {
            let obs_tx = c.get(&h).unwrap();
            assert_eq!(obs_tx.hash, h);
            if has_body {
                assert_eq!(obs_tx.transaction.clone().unwrap().hash, h);
            } else {
                assert!(obs_tx.transaction.is_none());
            }
            assert_eq!(obs_tx.interval, interval);
            assert_eq!(obs_tx.disappeared, disappeared);
        }
    }

    #[test]
    fn test_tx_new() {
        let obs_tx = ObservedTransaction::new(H1, 10);
        assert_eq!(obs_tx.hash, H1);
        assert!(obs_tx.transaction.is_none());
        assert_eq!(obs_tx.interval, (10, 10));
        assert!(obs_tx.disappeared.is_none());
    }

    #[test]
    fn test_tx_observe_expands_interval() {
        let mut obs_tx = ObservedTransaction::new(H1, 10);
        obs_tx.observe_at(20);
        assert_eq!(obs_tx.interval, (10, 20));
        obs_tx.observe_at(5);
        assert_eq!(obs_tx.interval, (5, 20));
        obs_tx.disappear_at(30);
        obs_tx.observe_at(25);
        assert_eq!(obs_tx.interval, (5, 25))
    }

    #[test]
    fn test_tx_disappear() {
        let mut obs_tx = ObservedTransaction::new(H1, 10);
        assert!(obs_tx.disappeared.is_none());
        obs_tx.disappear_at(20);
        assert_eq!(obs_tx.disappeared, Some(20));
        obs_tx.disappear_at(25);
        assert_eq!(obs_tx.disappeared, Some(20));
        obs_tx.disappear_at(15);
        assert_eq!(obs_tx.disappeared, Some(15));
    }

    #[test]
    fn test_tx_interval_disappear_overlap() {
        let mut obs_tx = ObservedTransaction::new(H1, 10);
        obs_tx.observe_at(20);
        obs_tx.disappear_at(15);
        assert_eq!(obs_tx.interval, (10, 15));
        assert_eq!(obs_tx.disappeared, Some(15));
    }

    #[test]
    fn test_tx_reset() {
        let mut obs_tx = ObservedTransaction::new(H1, 10);
        obs_tx.disappear_at(15);
        obs_tx.observe_at(15);
        assert_eq!(obs_tx.interval, (15, 15));
        assert!(obs_tx.disappeared.is_none());
    }

    #[test]
    fn test_tx_is_visible() {
        let mut obs_tx = ObservedTransaction::new(H1, 10);
        assert!(obs_tx.is_visible_at(10));
        assert!(!obs_tx.is_visible_at(9));
        assert!(obs_tx.is_visible_at(100));

        obs_tx.observe_at(20);
        assert!(obs_tx.is_visible_at(10));
        assert!(obs_tx.is_visible_at(100));
        assert!(!obs_tx.is_visible_at(9));

        obs_tx.disappear_at(30);
        assert!(obs_tx.is_visible_at(29));
        assert!(!obs_tx.is_visible_at(30));
    }

    #[test]
    fn test_tx_has_disappeared() {
        let mut obs_tx = ObservedTransaction::new(H1, 10);
        obs_tx.disappear_at(20);
        assert!(!obs_tx.has_disappeared_at(19));
        assert!(obs_tx.has_disappeared_at(20));
    }

    #[test]
    fn test_observe() {
        let mut p = Pool::new();
        p.observe(10, make_pool(vec![H1, H2]));
        p.observe(20, make_pool(vec![H1]));

        assert_content(p.content_at(9), vec![].into_iter().collect());
        assert_content(
            p.content_at(10),
            vec![(H1, true, (10, 20), None), (H2, true, (10, 10), Some(20))]
                .into_iter()
                .collect(),
        );
        assert_content(
            p.content_at(20),
            vec![(H1, true, (10, 20), None)].into_iter().collect(),
        );
    }

    #[test]
    fn test_pre_announce() {
        let mut p = Pool::new();
        p.pre_announce_transaction(10, H1);
        assert_content(
            p.content_at(10),
            vec![(H1, false, (10, 10), None)].into_iter().collect(),
        );
    }

    #[test]
    fn test_pre_announce_backfill() {
        let mut p = Pool::new();
        p.pre_announce_transaction(10, H1);
        p.observe(20, make_pool(vec![H1]));
        assert_content(
            p.content_at(10),
            vec![(H1, true, (10, 20), None)].into_iter().collect(),
        );
    }

    #[test]
    fn test_prune() {
        let mut p = Pool::new();
        p.observe(10, make_pool(vec![H1, H2]));
        p.observe(20, make_pool(vec![H1]));
        p.observe(30, make_pool(vec![]));
        assert_eq!(p.content_at(10).len(), 2);

        p.prune(19);
        assert_eq!(p.content_at(10).len(), 2);

        p.prune(20);
        assert_eq!(p.content_at(10).len(), 1);
        assert_content(
            p.content_at(10),
            vec![(H1, true, (10, 20), Some(30))].into_iter().collect(),
        );
    }
}
