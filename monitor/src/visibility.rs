use std::collections::{vec_deque, VecDeque};

use crate::types::Timestamp;

/// Observation represents a check if an item is visible or not at a certain
/// point in time.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Observation {
    Seen(Timestamp),
    NotSeen(Timestamp),
}

impl Observation {
    pub fn timestamp(&self) -> Timestamp {
        match self {
            Observation::Seen(t) => *t,
            Observation::NotSeen(t) => *t,
        }
    }
}

/// Visibility enumerates the different phases in which an item can be in:
///
/// - visible: between two Seen observations
/// - invisible: after a NotSeen observation (or not seen yet)
/// - disappearing: between a Seen and a NotSeen observation
///
/// There is no phase Appearing because we assume we notice immediately when
/// items are seen, so the duration in which an item would be Appearing is
/// negligible. In contrast, noticing that an item has disappeared may be
/// delayed, so the Disappearing phase is relevant.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Visibility {
    Visible {
        first_seen: Timestamp,
        last_seen: Timestamp,
    },
    Disappearing {
        first_seen: Timestamp,
        last_seen: Timestamp,
        disappeared: Timestamp,
    },
    Invisible {
        disappeared: Option<Timestamp>,
    },
}

/// Observation represents a sequence of observations made over time of a single
/// item.
#[derive(Debug, PartialEq)]
pub struct Observations(VecDeque<Observation>);

impl Observations {
    /// Create new empty Observations.
    pub fn new() -> Self {
        Observations(VecDeque::new())
    }

    /// Check if the item has not been observed at all (or all observations have
    /// been pruned).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the visibility at a certain timestamp.
    pub fn visibility_at(&self, timestamp: Timestamp) -> Visibility {
        // i is the index of the first observation after timestamp
        let i = self.0.partition_point(|&o| o.timestamp() <= timestamp);

        let obs_after = self.0.get(i);
        let obs_before = i.checked_sub(1).map_or(None, |j| self.0.get(j));
        let obs_two_before = i.checked_sub(2).map_or(None, |j| self.0.get(j));

        match obs_before {
            // if the item hasn't been observed yet at all, it's invisible
            None => Visibility::Invisible { disappeared: None },
            // if the item has just been observed as NotSeen, it's invisible
            Some(Observation::NotSeen(t)) => Visibility::Invisible {
                disappeared: Some(*t),
            },
            // if the item has been observed as Seen, it's either visible or disappearing, depending
            // on the following observation
            Some(Observation::Seen(t_before)) => {
                // We only ever keep the first and the last Seen observations, every Seen in
                // between will be squashed. Therefore, if the two previous observations are
                // Seens, the first seen timestamp is the one of obs_two_before.
                // Otherwise, it's the one of obs_before.
                let first_seen = if let Some(Observation::Seen(t_two_before)) = obs_two_before {
                    *t_two_before
                } else {
                    *t_before
                };
                match obs_after {
                    None => Visibility::Visible {
                        first_seen,
                        last_seen: *t_before,
                    },
                    Some(Observation::Seen(t_after)) => Visibility::Visible {
                        first_seen,
                        last_seen: *t_after,
                    },
                    Some(Observation::NotSeen(t_after)) => Visibility::Disappearing {
                        first_seen,
                        last_seen: *t_before,
                        disappeared: *t_after,
                    },
                }
            }
        }
    }

    /// Insert an observation, making sure invariants remain fulfilled, i.e.:
    ///   - No two NotSeens in a row (remove the second one)
    ///   - No three Seens in a row (delete the middle one)
    pub fn insert(&mut self, obs: Observation) {
        let i = self
            .0
            .partition_point(|&o| o.timestamp() <= obs.timestamp());
        self.0.insert(i, obs);

        // Check if we now have two NotSeens in a row (and if so delete the second one)
        // or three Seens in a row (and delete the middle one)
        match obs {
            Observation::NotSeen(_) => {
                let pre = i.checked_sub(1).map_or(None, |j| self.0.get(j));
                let post = self.0.get(i + 1);
                if pre.is_none() || matches!(pre, Some(Observation::NotSeen(_))) {
                    self.0.remove(i);
                } else if matches!(post, Some(Observation::NotSeen(_))) {
                    self.0.remove(i + 1);
                }
            }
            Observation::Seen(_) => {
                // Check if we now have three Seens in a row and if so delete the middle one
                let first_triple_start_index = i.saturating_sub(2);
                let last_triple_start_index = (i + 1).clamp(0, self.0.len().saturating_sub(2));
                for triple_start_index in first_triple_start_index..last_triple_start_index {
                    let mut triple_indices = triple_start_index..triple_start_index + 3;
                    let all_seens =
                        triple_indices.all(|j| matches!(self.0[j], Observation::Seen(_)));
                    if all_seens {
                        self.0.remove(triple_start_index + 1);
                        break;
                    }
                }
            }
        }
    }

    /// Remove old and unnecessary observations assuming we're only interested
    /// in visibilities at or later than cutoff.
    ///
    /// The only information that might be lost is the timestamp in an early
    /// NotSeen.
    pub fn prune(&mut self, cutoff: Timestamp) {
        // keep the observation right before the cutoff, remove all earlier ones as they
        // don't affect visibility after the cutoff
        while self.0.len() >= 2 && self.0[1].timestamp() <= cutoff {
            self.0.pop_front();
        }
        // remove leading NotSeen observations (even if they happened after the cutoff)
        // as they carry no information
        while self.0.len() >= 1 && matches!(self.0[0], Observation::NotSeen(_)) {
            self.0.pop_front();
        }
    }
}

impl FromIterator<Observation> for Observations {
    /// Create from a sequence of observations. The order of observations
    /// matters as they might be squashed during insert (see insert).
    fn from_iter<I: IntoIterator<Item = Observation>>(iter: I) -> Self {
        let mut obs = Observations::new();
        for o in iter {
            obs.insert(o);
        }
        obs
    }
}

impl IntoIterator for Observations {
    type IntoIter = vec_deque::IntoIter<Self::Item>;
    type Item = Observation;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn seen(t: Timestamp) -> Observation {
        Observation::Seen(t)
    }
    fn not_seen(t: Timestamp) -> Observation {
        Observation::NotSeen(t)
    }

    #[test]
    fn test_visibility_at() {
        struct TestCase {
            obs: Vec<Observation>,
            t: Timestamp,
            v: Visibility,
        }

        let test_cases = vec![
            TestCase {
                obs: vec![],
                t: 0,
                v: Visibility::Invisible { disappeared: None },
            },
            TestCase {
                obs: vec![seen(10)],
                t: 9,
                v: Visibility::Invisible { disappeared: None },
            },
            TestCase {
                obs: vec![seen(10)],
                t: 10,
                v: Visibility::Visible {
                    first_seen: 10,
                    last_seen: 10,
                },
            },
            TestCase {
                obs: vec![seen(10), not_seen(15)],
                t: 10,
                v: Visibility::Disappearing {
                    first_seen: 10,
                    last_seen: 10,
                    disappeared: 15,
                },
            },
            TestCase {
                obs: vec![seen(10), seen(15), not_seen(20)],
                t: 15,
                v: Visibility::Disappearing {
                    first_seen: 10,
                    last_seen: 15,
                    disappeared: 20,
                },
            },
            TestCase {
                obs: vec![seen(10), not_seen(15)],
                t: 15,
                v: Visibility::Invisible {
                    disappeared: Some(15),
                },
            },
            TestCase {
                obs: vec![seen(10), seen(15)],
                t: 10,
                v: Visibility::Visible {
                    first_seen: 10,
                    last_seen: 15,
                },
            },
            TestCase {
                obs: vec![seen(10), seen(15)],
                t: 15,
                v: Visibility::Visible {
                    first_seen: 10,
                    last_seen: 15,
                },
            },
        ];

        for test_case in test_cases {
            let obs = Observations::from_iter(test_case.obs);
            let v = obs.visibility_at(test_case.t);
            assert_eq!(v, test_case.v);
        }
    }

    #[test]
    fn test_insert() {
        struct TestCase {
            obs: Vec<Observation>,
            o: Observation,
            exp: Vec<Observation>,
        }
        let test_cases = vec![
            TestCase {
                obs: vec![],
                o: seen(10),
                exp: vec![seen(10)],
            },
            TestCase {
                obs: vec![],
                o: not_seen(10),
                exp: vec![],
            },
            TestCase {
                obs: vec![seen(10)],
                o: seen(20),
                exp: vec![seen(10), seen(20)],
            },
            TestCase {
                obs: vec![seen(20)],
                o: seen(10),
                exp: vec![seen(10), seen(20)],
            },
            TestCase {
                obs: vec![seen(10), not_seen(20)],
                o: not_seen(30),
                exp: vec![seen(10), not_seen(20)],
            },
            TestCase {
                obs: vec![seen(10), not_seen(30)],
                o: not_seen(20),
                exp: vec![seen(10), not_seen(20)],
            },
            TestCase {
                obs: vec![seen(10), seen(20)],
                o: seen(30),
                exp: vec![seen(10), seen(30)],
            },
            TestCase {
                obs: vec![seen(10), seen(30)],
                o: seen(20),
                exp: vec![seen(10), seen(30)],
            },
            TestCase {
                obs: vec![seen(20), seen(30)],
                o: seen(10),
                exp: vec![seen(10), seen(30)],
            },
        ];
        for test_case in test_cases {
            let mut obs = Observations::new();
            for o in test_case.obs {
                obs.insert(o);
            }
            obs.insert(test_case.o);
            assert!(obs.into_iter().eq(test_case.exp));
        }
    }

    #[test]
    fn test_prune() {
        struct TestCase {
            obs: Vec<Observation>,
            t: Timestamp,
            n_pruned: usize,
        }
        let test_cases = [
            TestCase {
                obs: vec![],
                t: 0,
                n_pruned: 0,
            },
            TestCase {
                obs: vec![seen(10)],
                t: 10,
                n_pruned: 0,
            },
            TestCase {
                obs: vec![seen(10)],
                t: 11,
                n_pruned: 0,
            },
            TestCase {
                obs: vec![not_seen(10)],
                t: 10,
                n_pruned: 1,
            },
            TestCase {
                obs: vec![not_seen(10)],
                t: 11,
                n_pruned: 1,
            },
            TestCase {
                obs: vec![seen(10), seen(20)],
                t: 19,
                n_pruned: 0,
            },
            TestCase {
                obs: vec![seen(10), seen(20)],
                t: 20,
                n_pruned: 1,
            },
            TestCase {
                obs: vec![seen(10), not_seen(20), seen(30)],
                t: 20,
                n_pruned: 2,
            },
        ];
        for test_case in test_cases {
            let mut obs = Observations::from_iter(test_case.obs.clone());
            obs.prune(test_case.t);
            let exp = test_case.obs[test_case.n_pruned..].into_iter().cloned();
            assert!(obs.into_iter().eq(exp));
        }
    }

    #[test]
    fn test_is_empty() {
        let empty = Observations::from_iter(vec![]);
        assert!(empty.is_empty());
        let non_empty = Observations::from_iter(vec![Observation::Seen(10)]);
        assert!(!non_empty.is_empty());
    }
}
