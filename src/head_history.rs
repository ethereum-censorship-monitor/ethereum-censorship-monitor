use std::collections::VecDeque;

use chrono::{DateTime, Duration, Utc};
use ethers::types::Transaction;

use crate::{metrics, types::BeaconBlock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedHead {
    pub head: BeaconBlock<Transaction>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug)]
pub struct HeadHistory(VecDeque<ObservedHead>);

/// HeadHistory stores the history of head blocks.
impl HeadHistory {
    pub fn new() -> Self {
        HeadHistory(VecDeque::new())
    }

    /// Insert a new block into the history observed at the given timestamp.
    pub fn observe(&mut self, timestamp: DateTime<Utc>, head: BeaconBlock<Transaction>) {
        let i = self.0.partition_point(|oh| oh.timestamp <= timestamp);
        let dt = timestamp - head.proposal_time();
        if dt < Duration::zero() {
            log::warn!(
                "received block {} {:2}s before proposal time",
                head,
                -dt.num_milliseconds() as f64 / 1000.
            );
        }
        log::debug!(
            "inserting block {} observed {:2}s after proposal time into head history at index {} \
             (current length {})",
            head,
            dt.num_milliseconds() as f64 / 1000.,
            i,
            self.0.len(),
        );
        self.0.insert(i, ObservedHead { head, timestamp });
        self.report();
    }

    /// Delete blocks that do not affect the history at or after cutoff.
    #[allow(dead_code)]
    pub fn prune(&mut self, cutoff: DateTime<Utc>) {
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
                    cutoff,
                );
                break;
            }
        }
        self.report();
    }

    /// Get the block we considered the head at the given time, if any.
    pub fn at(&self, timestamp: DateTime<Utc>) -> Option<ObservedHead> {
        // i is the index of the first block after t. We're interested in the one right
        // before
        let i = self.0.partition_point(|oh| oh.timestamp <= timestamp);
        let j = i.checked_sub(1)?;
        let oh = self.0.get(j)?;
        assert!(oh.timestamp <= timestamp);
        Some(oh.clone())
    }

    fn report(&self) {
        metrics::HEAD_HISTORY_LENGTH.set(self.0.len() as i64);
    }
}

#[cfg(test)]
mod test {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::types::{H256, U64};

    fn new_block(slot: u64) -> BeaconBlock<Transaction> {
        let mut b = BeaconBlock::default();
        b.slot = U64::from(slot);
        b.root = H256::from_low_u64_be(slot);
        b
    }

    #[test]
    fn test() {
        let mut h = HeadHistory::new();

        let t0 = Utc.timestamp_opt(0, 0).unwrap();
        let t1 = Utc.timestamp_opt(10, 0).unwrap();
        let t2 = Utc.timestamp_opt(20, 0).unwrap();
        let t3 = Utc.timestamp_opt(30, 0).unwrap();
        let one_sec = Duration::seconds(1);

        assert_eq!(h.at(t0), None);

        let b0 = new_block(0);
        let b1 = new_block(1);
        let b2 = new_block(2);

        let o0 = ObservedHead {
            head: b0,
            timestamp: t1,
        };
        let o1 = ObservedHead {
            head: b2,
            timestamp: t2,
        };
        let o2 = ObservedHead {
            head: b1,
            timestamp: t3,
        };

        for o in vec![&o0, &o1, &o2] {
            h.observe(o.timestamp, o.head.clone());
        }

        assert!(h.at(t0).is_none());
        assert!(h.at(t0 - one_sec).is_none());
        assert_eq!(h.at(t1).unwrap(), o0);
        assert_eq!(h.at(t2 - one_sec).unwrap(), o0);
        assert_eq!(h.at(t2).unwrap(), o1);
        assert_eq!(h.at(t3 - one_sec).unwrap(), o1);
        assert_eq!(h.at(t3).unwrap(), o2);
        assert_eq!(h.at(t3 + one_sec).unwrap(), o2);

        h.prune(t3 - one_sec);

        assert!(h.at(t0).is_none());
        assert!(h.at(t1 - one_sec).is_none());
        assert!(h.at(t2 - one_sec).is_none());
        assert_eq!(h.at(t2).unwrap(), o1);
        assert_eq!(h.at(t3 - one_sec).unwrap(), o1);
        assert_eq!(h.at(t3).unwrap(), o2);
        assert_eq!(h.at(t3 + one_sec).unwrap(), o2);
    }
}
