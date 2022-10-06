use log::warn;
use std::collections::{HashMap, HashSet};

use crate::types::{Timestamp, TxHash};
use crate::visibility::{ChronologyError, Observation, Observations, Visibility};

/// This struct keeps track of the current tx pool and how it changes over time. It provides
/// a method to query the set of transactions and their visibilities that were present at a given
/// point in time.
pub struct Pool {
    timestamp: Timestamp,
    content: HashSet<TxHash>,
    tx_obs: HashMap<TxHash, Observations>,
}

impl Pool {
    pub fn new() -> Self {
        Pool {
            timestamp: 0,
            content: HashSet::new(),
            tx_obs: HashMap::new(),
        }
    }

    /// Inform the pool about the exact set of transactions it should contain. The transactions
    /// will be observed as Seen. The transactions that are missing compared to the previous
    /// invocation (or intermediate additions via pre_announce_transaction) are observed as
    /// NotSeen.
    pub fn observe(
        &mut self,
        t: Timestamp,
        content: HashSet<TxHash>,
    ) -> Result<(), ChronologyError> {
        if t < self.timestamp {
            return Err(ChronologyError);
        }

        let unseen_hashes = self.content.difference(&content);
        let unseen_hashes: HashSet<TxHash> = unseen_hashes.copied().collect();
        self.timestamp = t;
        self.content = content;

        for hash in &self.content {
            let r = self
                .tx_obs
                .entry(hash.clone())
                .or_insert_with(Observations::new)
                .append(Observation::Seen(t));
            if let Err(_) = r {
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
        if t < self.timestamp {
            return Err(ChronologyError);
        }
        self.timestamp = t;

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
    pub fn content_at(&self, t: Timestamp) -> HashMap<TxHash, Visibility> {
        let mut txs: HashMap<TxHash, Visibility> = HashMap::new();
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
                txs.insert(tx_hash.clone(), vis);
            }
        }
        txs
    }

    pub fn prune(&mut self, cutoff: Timestamp) {
        for (_, obs) in self.tx_obs.iter_mut() {
            obs.prune(cutoff);
        }
    }
}

mod test {
    use super::*;

    const H1: TxHash = TxHash::repeat_byte(1);
    const H2: TxHash = TxHash::repeat_byte(2);

    #[test]
    fn test_observe() {
        let mut p = Pool::new();
        p.observe(10, vec![H1, H2].into_iter().collect()).unwrap();
        p.observe(20, vec![H1].into_iter().collect()).unwrap();

        assert_eq!(p.content_at(9), vec![].into_iter().collect());
        assert_eq!(
            p.content_at(10),
            vec![
                (
                    H1,
                    Visibility::Visible {
                        first_seen: 10,
                        last_seen: 20,
                    }
                ),
                (
                    H2,
                    Visibility::Disappearing {
                        first_seen: 10,
                        last_seen: 10,
                        disappeared: 20
                    }
                )
            ]
            .into_iter()
            .collect()
        );
        assert_eq!(
            p.content_at(20),
            vec![(
                H1,
                Visibility::Visible {
                    first_seen: 10,
                    last_seen: 20
                }
            )]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn test_pre_announce() {
        let mut p = Pool::new();
        p.pre_announce_transaction(10, H1).unwrap();
        assert_eq!(
            p.content_at(10),
            vec![(
                H1,
                Visibility::Visible {
                    first_seen: 10,
                    last_seen: 10
                }
            )]
            .into_iter()
            .collect()
        );

        p.pre_announce_transaction(9, H2).unwrap_err();
    }
}
