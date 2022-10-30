use crate::types::{ChronologyError, Timestamp};
use std::collections::VecDeque;

/// History keeps track of how a value changes over time.
pub struct History<T>(VecDeque<(Timestamp, T)>);

impl<T> History<T> {
    /// Create a new empty head history.
    pub fn new() -> Self {
        History(VecDeque::new())
    }

    /// Add a new value observed at the given time which must be at the same time or later than the
    /// previously added one.
    pub fn append(&mut self, t: Timestamp, v: T) -> Result<(), ChronologyError> {
        if self.0.len() > 0 && t < self.0[self.0.len() - 1].0 {
            return Err(ChronologyError);
        }
        self.0.push_back((t, v));
        Ok(())
    }

    /// Remove values that don't affect the value at or after the given cutoff timestamp (i.e.,
    /// remove all but one values with timestamps smaller or equal to cutoff).
    pub fn prune(&mut self, cutoff: Timestamp) {
        while self.0.len() >= 2 && self.0[0].0 <= cutoff && self.0[1].0 <= cutoff {
            self.0.pop_front();
        }
    }

    /// Get the head at the given time.
    pub fn at(&self, t: Timestamp) -> Option<&T> {
        let i = self.0.partition_point(|(s, _)| s <= &t);
        if i == 0 {
            None
        } else {
            self.0.get(i - 1).map(|(_, b)| b)
        }
    }
}

mod test {
    use super::*;

    #[test]
    fn test() {
        let mut h: History<&str> = History::new();
        assert!(h.at(0).is_none());

        h.append(10, "one").unwrap();
        assert!(h.at(9).is_none());
        assert_eq!(h.at(10).unwrap(), &"one");
        assert_eq!(h.at(20).unwrap(), &"one");

        h.append(20, "two").unwrap();
        assert_eq!(h.at(19).unwrap(), &"one");
        assert_eq!(h.at(20).unwrap(), &"two");

        h.append(20, "three").unwrap();
        assert_eq!(h.at(19).unwrap(), &"one");
        assert_eq!(h.at(20).unwrap(), &"three");

        h.append(19, "err").unwrap_err();

        h.prune(19);
        assert_eq!(h.at(15).unwrap(), &"one");
        h.prune(20);
        assert!(h.at(15).is_none());
        assert_eq!(h.at(20).unwrap(), &"three");
    }

    #[test]
    fn test2() {
        let mut h: History<String> = History::new();

        {
            let s = String::from("hi");
            h.append(10, s).unwrap();
        }
        assert_eq!(h.at(10).unwrap(), "hi");
    }
}
