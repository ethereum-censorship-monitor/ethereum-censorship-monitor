use std::{cmp, collections::BTreeMap};

use chrono::{naive::serde::ts_seconds, NaiveDateTime};
use serde::Serialize;

use super::Miss;

#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct Block {
    pub block_hash: String,
    pub slot: i32,
    pub block_number: i32,
    #[serde(with = "ts_seconds")]
    pub proposal_time: NaiveDateTime,
    pub proposer_index: i32,
    pub num_misses: usize,
    pub misses: Vec<BlockMiss>,
}

#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct BlockMiss {
    pub tx_hash: String,
    #[serde(with = "ts_seconds")]
    pub tx_first_seen: NaiveDateTime,
    #[serde(with = "ts_seconds")]
    pub tx_quorum_reached: NaiveDateTime,
    pub tip: Option<i64>,
    pub sender: String,
}

impl Block {
    fn insert(&mut self, miss: &Miss) {
        let block_miss: BlockMiss = miss.into();
        let index = self.misses.binary_search(&block_miss).unwrap_or_else(|i| i);
        self.misses.insert(index, block_miss);
        self.num_misses += 1;
    }
}

impl From<&Miss> for Block {
    fn from(miss: &Miss) -> Self {
        Self {
            block_hash: miss.block_hash.clone(),
            slot: miss.slot,
            block_number: miss.block_number,
            proposal_time: miss.proposal_time,
            proposer_index: miss.proposer_index,
            num_misses: 1,
            misses: vec![miss.into()],
        }
    }
}

impl From<&Miss> for BlockMiss {
    fn from(miss: &Miss) -> Self {
        Self {
            tx_hash: miss.tx_hash.clone(),
            tx_first_seen: miss.tx_first_seen,
            tx_quorum_reached: miss.tx_quorum_reached,
            sender: miss.sender.clone(),
            tip: miss.tip,
        }
    }
}

impl Block {
    fn cmp_tuple(&self) -> (NaiveDateTime, Option<NaiveDateTime>) {
        (
            self.proposal_time,
            self.misses.get(0).map(|m| m.tx_quorum_reached),
        )
    }
}

impl cmp::Ord for Block {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.cmp_tuple().cmp(&other.cmp_tuple())
    }
}

impl cmp::PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for BlockMiss {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.tx_quorum_reached.cmp(&other.tx_quorum_reached)
    }
}

impl cmp::PartialOrd for BlockMiss {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn group_misses_to_blocks(misses: &[Miss]) -> Vec<Block> {
    let blocks_btree: BTreeMap<String, Block> =
        misses.iter().fold(BTreeMap::new(), |mut acc, miss| {
            acc.entry(miss.block_hash.clone())
                .and_modify(|block| block.insert(miss))
                .or_insert_with(|| Block::from(miss));
            acc
        });
    let mut blocks: Vec<Block> = blocks_btree.values().cloned().collect();
    blocks.sort();
    blocks
}
