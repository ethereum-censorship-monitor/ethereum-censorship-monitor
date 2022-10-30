use crate::types::{Block, ChronologyError, Timestamp, H256};
use log;
use std::collections::VecDeque;

pub struct HeadHistory(VecDeque<(Timestamp, Block<H256>)>);

/// HeadHistory stores the history of head blocks. In the event of a reorg or missed blocks (that
/// is, the new block is not a child of the current head), the whole history is cleared and only
/// the new head remains. The rationale here is that in these cases we likely had an incorrect view
/// of the network, so starting fresh is safest. While it might be possible to recover, reorgs are
/// too infrequent to justify the increased complexity.
impl HeadHistory {
    pub fn new() -> Self {
        HeadHistory(VecDeque::new())
    }

    /// Add a new block to the history observed at the given timestamp. If the block is not the
    /// child of the current head, the history will be cleared.
    pub fn observe(&mut self, t: Timestamp, head: Block<H256>) -> Result<(), ChronologyError> {
        if let Some((last_t, last_head)) = self.0.back() {
            if t < *last_t {
                return Err(ChronologyError {});
            }
            if head.parent_hash != last_head.hash.expect("all blocks should have a hash") {
                log::info!(
                    "clearing head history due to reorg or missed blocks from {} to {}",
                    last_head.hash.unwrap(),
                    head.hash.unwrap()
                );
                self.0.clear();
            }
        }
        self.0.push_back((t, head));
        Ok(())
    }

    /// Delete blocks that do not affect the history at or after cutoff.
    pub fn prune(&mut self, cutoff: Timestamp) {
        while let Some((t1, _)) = self.0.get(1) {
            if *t1 <= cutoff {
                self.0.pop_front();
            } else {
                break;
            }
        }
    }

    /// Get the block we considered the head at the given time, if any.
    pub fn at(&self, t: Timestamp) -> Option<Block<H256>> {
        // i is the index of the first block after t. We're interested in the one right before
        let i = self.0.partition_point(|(t0, _)| *t0 <= t);
        let j = i.checked_sub(1)?;
        let (t0, b) = self.0.get(j)?;
        assert!(*t0 <= t);
        Some(b.clone())
    }
}

mod test {
    use super::*;

    fn new_block(n: u64) -> Block<H256> {
        let mut b = Block::default();
        b.number = Some(ethers::types::U64::from(n));
        b.hash = Some(H256::from_low_u64_be(n));
        b
    }

    fn new_child(b: &Block<H256>) -> Block<H256> {
        let mut child = b.clone();
        child.number = Some(child.number.unwrap() + 1);
        child.hash = Some(H256::from_low_u64_be(child.number.unwrap().as_u64()));
        child.parent_hash = b.hash.unwrap();
        child
    }

    #[test]
    fn test_new() {
        let h = HeadHistory::new();
        assert_eq!(h.at(0), None);
    }

    #[test]
    fn test_prune() {
        let mut h = HeadHistory::new();
        let b1 = new_block(0);
        let b2 = new_child(&b1);
        let b3 = new_child(&b2);
        h.observe(10, b1).unwrap();
        h.observe(20, b2).unwrap();
        h.observe(30, b3).unwrap();
        h.prune(29);
        assert!(h.at(19).is_none());
        assert!(h.at(20).is_some());
    }

    #[test]
    fn test_at_empty() {
        let h = HeadHistory::new();
        assert!(h.at(0).is_none());
    }

    #[test]
    fn test_at_one() {
        let mut h = HeadHistory::new();
        let b = new_block(0);
        h.observe(10, b.clone()).unwrap();
        assert!(h.at(9).is_none());
        assert_eq!(h.at(10).unwrap(), b);
        assert_eq!(h.at(20).unwrap(), b);
    }

    #[test]
    fn test_at_two() {
        let mut h = HeadHistory::new();
        let b1 = new_block(0);
        let b2 = new_child(&b1);
        h.observe(10, b1.clone()).unwrap();
        h.observe(20, b2.clone()).unwrap();
        assert!(h.at(9).is_none());
        assert_eq!(h.at(10).unwrap(), b1);
        assert_eq!(h.at(19).unwrap(), b1);
        assert_eq!(h.at(20).unwrap(), b2);
        assert_eq!(h.at(29).unwrap(), b2);
    }

    #[test]
    fn test_observe() {
        let mut h = HeadHistory::new();
        let num_blocks = 10;
        let mut blocks_and_times: Vec<(Block<H256>, u64)> = vec![(new_block(1), 10)];
        while blocks_and_times.len() < num_blocks {
            let (parent, t0) = &blocks_and_times[blocks_and_times.len() - 1];
            let child = new_child(&parent);
            let t1 = t0 + 10;
            blocks_and_times.push((child, t1));
        }
        for (b, t) in &blocks_and_times {
            h.observe(*t, b.clone()).unwrap();
        }
        assert!(h.at(9).is_none());
        for (b, t0) in &blocks_and_times {
            let t0 = *t0;
            let ts = vec![t0, t0 + 1, t0 + 9];
            for t in ts {
                println!("{}, {}", t, b.number.unwrap());
                assert_eq!(h.at(t), Some(b.clone()));
            }
        }
        assert_eq!(
            h.at(u64::MAX),
            Some(blocks_and_times[num_blocks - 1].0.clone())
        );
    }

    #[test]
    fn test_observe_reorg() {
        let mut h = HeadHistory::new();
        let b1 = new_block(1);
        let b2 = new_block(1);
        h.observe(10, b1.clone()).unwrap();
        h.observe(20, b2.clone()).unwrap();
        assert!(h.at(19).is_none());
        assert_eq!(h.at(20).unwrap(), b2);
    }
}
