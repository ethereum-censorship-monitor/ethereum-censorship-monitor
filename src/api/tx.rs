use std::{cmp, collections::BTreeMap};

use chrono::{naive::serde::ts_seconds, NaiveDateTime};
use serde::Serialize;

use super::Miss;

#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct Tx {
    pub tx_hash: String,
    #[serde(with = "ts_seconds")]
    pub tx_first_seen: NaiveDateTime,
    #[serde(with = "ts_seconds")]
    pub tx_quorum_reached: NaiveDateTime,
    pub sender: String,
    pub num_misses: usize,
    pub misses: Vec<TxMiss>,
}

#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct TxMiss {
    pub block_hash: String,
    pub slot: i32,
    pub block_number: i32,
    #[serde(with = "ts_seconds")]
    pub proposal_time: NaiveDateTime,
    pub proposer_index: i32,
    pub tip: Option<i64>,
}

impl Tx {
    fn insert(&mut self, miss: &Miss) {
        let tx_miss: TxMiss = miss.into();
        let index = self.misses.binary_search(&tx_miss).unwrap_or_else(|i| i);
        self.misses.insert(index, tx_miss);
        self.num_misses += 1;
    }
}

impl From<&Miss> for Tx {
    fn from(miss: &Miss) -> Self {
        Self {
            tx_hash: miss.tx_hash.clone(),
            tx_first_seen: miss.tx_first_seen,
            tx_quorum_reached: miss.tx_quorum_reached,
            sender: miss.sender.clone(),
            num_misses: 1,
            misses: vec![miss.into()],
        }
    }
}

impl From<&Miss> for TxMiss {
    fn from(miss: &Miss) -> Self {
        Self {
            block_hash: miss.block_hash.clone(),
            slot: miss.slot,
            block_number: miss.block_number,
            proposal_time: miss.proposal_time,
            proposer_index: miss.proposer_index,
            tip: miss.tip,
        }
    }
}

impl Tx {
    fn cmp_tuple(&self) -> (Option<NaiveDateTime>, NaiveDateTime) {
        (
            self.misses.get(0).map(|m| m.proposal_time),
            self.tx_quorum_reached,
        )
    }
}

impl cmp::Ord for Tx {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.cmp_tuple().cmp(&other.cmp_tuple())
    }
}

impl cmp::PartialOrd for Tx {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for TxMiss {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.proposal_time.cmp(&other.proposal_time)
    }
}

impl cmp::PartialOrd for TxMiss {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn group_misses_to_txs(misses: &[Miss]) -> Vec<Tx> {
    let txs_btree: BTreeMap<String, Tx> = misses.iter().fold(BTreeMap::new(), |mut acc, miss| {
        acc.entry(miss.tx_hash.clone())
            .and_modify(|tx| tx.insert(miss))
            .or_insert_with(|| Tx::from(miss));
        acc
    });
    let mut txs: Vec<Tx> = txs_btree.values().cloned().collect();
    txs.sort();
    txs
}
