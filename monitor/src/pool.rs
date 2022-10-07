use ethers::types::TxpoolContent;
use log::warn;
use std::collections::{HashMap, HashSet};

use crate::types::{Timestamp, TxHash, TxpoolTransaction};
use crate::visibility::{ChronologyError, Observation, Observations, Visibility};

#[derive(Debug)]
pub struct TransactionWithVisibility {
    pub hash: TxHash,
    pub transaction: Option<TxpoolTransaction>,
    pub visibility: Visibility,
}

/// This struct keeps track of the current tx pool and how it changes over time. It provides
/// a method to query the set of transactions and their visibilities that were present at a given
/// point in time.
pub struct Pool {
    last_timestamp: Timestamp,
    last_content: HashSet<TxHash>,
    tx_obs: HashMap<TxHash, Observations>,
    txs: HashMap<TxHash, TxpoolTransaction>,
}

impl Pool {
    pub fn new() -> Self {
        Pool {
            last_timestamp: 0,
            last_content: HashSet::new(),
            tx_obs: HashMap::new(),
            txs: HashMap::new(),
        }
    }

    /// Inform the pool about the exact set of transactions it should contain. The transactions
    /// will be observed as Seen. The transactions that are missing compared to the previous
    /// invocation (or intermediate additions via pre_announce_transaction) are observed as
    /// NotSeen.
    pub fn observe(&mut self, t: Timestamp, content: TxpoolContent) -> Result<(), ChronologyError> {
        if t < self.last_timestamp {
            return Err(ChronologyError);
        }

        let mut txs: HashMap<TxHash, &TxpoolTransaction> = HashMap::new();
        for v in content.pending.values().chain(content.queued.values()) {
            for tx in v.values() {
                txs.insert(tx.hash, tx);
            }
        }
        let hashes: HashSet<TxHash> = txs.keys().copied().collect();

        let unseen_hashes = self.last_content.difference(&hashes);
        let unseen_hashes: HashSet<TxHash> = unseen_hashes.copied().collect();
        self.last_timestamp = t;
        self.last_content = hashes;

        for hash in &self.last_content {
            let r = self
                .tx_obs
                .entry(hash.clone())
                .or_insert_with(Observations::new)
                .append(Observation::Seen(t));
            if let Ok(_) = r {
                self.txs.insert(*hash, (*txs.get(hash).unwrap()).clone());
            } else {
                warn!("ignoring non-chronological pool observation");
            }
        }
        for hash in unseen_hashes {
            if let Some(obs) = self.tx_obs.get_mut(&hash) {
                let r = obs.append(Observation::NotSeen(t));
                if let Err(_) = r {
                    warn!("ignoring non-chronological pool observation");
                }
            }
        }

        Ok(())
    }

    /// Inform the pool about an individual transaction observed as Seen at the given timestamp.
    pub fn pre_announce_transaction(
        &mut self,
        t: Timestamp,
        hash: TxHash,
    ) -> Result<(), ChronologyError> {
        if t < self.last_timestamp {
            return Err(ChronologyError);
        }
        self.last_timestamp = t;

        let r = self
            .tx_obs
            .entry(hash.clone())
            .or_insert_with(Observations::new)
            .append(Observation::Seen(t));
        if let Err(_) = r {
            warn!("ignoring non-chronological transaction pre-announce");
        }
        Ok(())
    }

    /// Query the set of transactions that are either visible or disappearing at the given
    /// timestamp.
    pub fn content_at(&self, t: Timestamp) -> HashMap<TxHash, TransactionWithVisibility> {
        let mut txs: HashMap<TxHash, TransactionWithVisibility> = HashMap::new();
        for (tx_hash, obs) in &self.tx_obs {
            let vis = obs.visibility_at(t);
            let insert = match vis {
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
            };
            if insert {
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

    pub fn prune(&mut self, cutoff: Timestamp) {
        let mut fully_pruned: HashSet<TxHash> = HashSet::new();
        for (tx_hash, obs) in self.tx_obs.iter_mut() {
            obs.prune(cutoff);
            if obs.is_empty() {
                fully_pruned.insert(*tx_hash);
            }
        }
        for tx_hash in fully_pruned {
            self.tx_obs.remove(&tx_hash);
            self.txs.remove(&tx_hash);
        }
    }
}

mod test {
    use std::{collections::BTreeMap, str::FromStr};

    use ethers::types::{Address, Bytes, U256};

    use super::*;

    const H1: TxHash = TxHash::repeat_byte(1);
    const H2: TxHash = TxHash::repeat_byte(2);

    fn make_pool(hashes: Vec<TxHash>) -> TxpoolContent {
        let queued = BTreeMap::new();

        let mut pending = BTreeMap::new();
        let mut txs = BTreeMap::new();
        let address = Address::repeat_byte(0);
        for (i, h) in hashes.iter().enumerate() {
            let tx = TxpoolTransaction {
                block_hash: None,
                block_number: None,
                from: None,
                gas: None,
                gas_price: None,
                hash: *h,
                input: Bytes::from([]),
                nonce: U256::from(0),
                to: None,
                transaction_index: None,
                value: U256::from(0),
            };
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
        p.observe(10, make_pool(vec![H1, H2])).unwrap();
        p.observe(20, make_pool(vec![H1])).unwrap();

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
        p.pre_announce_transaction(10, H1).unwrap();
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

        p.pre_announce_transaction(9, H2).unwrap_err();
    }
}