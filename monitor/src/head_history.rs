use crate::types::{BeaconBlock, Timestamp};
use ethers::types::Transaction;
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub struct ObservedHead {
    pub head: BeaconBlock<Transaction>,
    pub timestamp: Timestamp,
}

#[derive(Debug)]
pub struct HeadHistory(VecDeque<ObservedHead>);

/// HeadHistory stores the history of head blocks.
impl HeadHistory {
    pub fn new() -> Self {
        HeadHistory(VecDeque::new())
    }

    /// Insert a new block into the history observed at the given timestamp.
    pub fn observe(&mut self, timestamp: Timestamp, head: BeaconBlock<Transaction>) {
        let i = self.0.partition_point(|oh| oh.timestamp <= timestamp);
        log::debug!(
            "inserting block {} observed at time {} ({}s after proposal time) into head history at \
            index {} (current length {})",
            head,
            timestamp,
            timestamp - head.proposal_time(),
            i,
            self.0.len(),
        );
        self.0.insert(i, ObservedHead { head, timestamp });
    }

    /// Delete blocks that do not affect the history at or after cutoff.
    pub fn prune(&mut self, cutoff: Timestamp) {
        let mut num_pruned = 0;
        while let Some(oh) = self.0.get(1) {
            if oh.timestamp <= cutoff {
                self.0.pop_front();
                num_pruned += 1;
            } else {
                log::debug!(
                    "pruned {} of {} blocks before time {} in head history",
                    num_pruned,
                    self.0.len() + num_pruned,
                    cutoff
                );
                break;
            }
        }
    }

    /// Get the block we considered the head at the given time, if any.
    pub fn at(&self, timestamp: Timestamp) -> Option<ObservedHead> {
        // i is the index of the first block after t. We're interested in the one right before
        let i = self.0.partition_point(|oh| oh.timestamp <= timestamp);
        let j = i.checked_sub(1)?;
        let oh = self.0.get(j)?;
        assert!(oh.timestamp <= timestamp);
        Some(oh.clone())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn new_block(slot: u64) -> BeaconBlock<Transaction> {
        let mut b = BeaconBlock::default();
        b.slot = U64::from(slot);
        b.root = H256::from_low_u64_be(slot);
        b
    }

    #[test]
    fn test() {
        let mut h = HeadHistory::new();

        assert_eq!(h.at(0), None);

        let b0 = new_block(10);
        let b1 = new_block(20);
        let b2 = new_block(30);

        let o0 = ObservedHead {
            head: b0,
            timestamp: 10,
        };
        let o1 = ObservedHead {
            head: b2,
            timestamp: 20,
        };
        let o2 = ObservedHead {
            head: b1,
            timestamp: 30,
        };

        for o in vec![&o0, &o1, &o2] {
            h.observe(o.timestamp, o.head.clone());
        }

        assert!(h.at(0).is_none());
        assert!(h.at(9).is_none());
        assert_eq!(h.at(10).unwrap(), o0);
        assert_eq!(h.at(19).unwrap(), o0);
        assert_eq!(h.at(20).unwrap(), o1);
        assert_eq!(h.at(29).unwrap(), o1);
        assert_eq!(h.at(30).unwrap(), o2);
        assert_eq!(h.at(300).unwrap(), o2);

        h.prune(29);

        assert!(h.at(0).is_none());
        assert!(h.at(9).is_none());
        assert!(h.at(19).is_none());
        assert_eq!(h.at(20).unwrap(), o1);
        assert_eq!(h.at(29).unwrap(), o1);
        assert_eq!(h.at(30).unwrap(), o2);
        assert_eq!(h.at(300).unwrap(), o2);
    }
}
