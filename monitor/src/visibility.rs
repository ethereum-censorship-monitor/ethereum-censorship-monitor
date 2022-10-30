use crate::types::Timestamp;
use std::collections;
use std::fmt;

/// Observation represents a check if an item is visible or not at a certain point in time.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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
/// There is no phase appearing because we assume we notice immediately when items are seen, but
/// noticing that an item disappeared may be delayed.
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

/// ChronologyError is returned when events are reported in wrong order.
#[derive(Debug, PartialEq, Eq)]
pub struct ChronologyError;

impl fmt::Display for ChronologyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "observations added in non-chronological order")
    }
}

/// Observation represents a sequence of observations made over time of a single item.
#[derive(Debug, PartialEq, Eq)]
pub struct Observations(collections::VecDeque<Observation>);

impl Observations {
    /// Create new empty Observations.
    pub fn new() -> Self {
        Observations(collections::VecDeque::new())
    }

    /// Create from a sequence of observations. The observations must be ordered properly,
    /// otherwise a ChronologyError is returned. Observations are automatically "squashed" (see the
    /// documentation to append).
    pub fn from_iter<I: IntoIterator<Item = Observation>>(
        iter: I,
    ) -> Result<Self, ChronologyError> {
        let mut obs = Observations::new();
        for o in iter {
            let r = obs.append(o);
            if let Err(e) = r {
                return Err(e);
            }
        }
        Ok(obs)
    }

    /// Check if the item has not been observed at all (or all observations have been pruned).
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
            // if the item has been seen as seen, it's either visible or disappearing, depending on
            // the following observation
            Some(Observation::Seen(t_before)) => {
                // We only ever keep the first and the last Seen observations, every Seen in
                // between will be squashed. Therefore, if the two previous observations are Seens,
                // the first seen timestamp is the one of obs_two_before. Otherwise, it's the one
                // of obs_before.
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

        // if obs_before.is_none() {
        //     return Visibility::Invisible { disappeared: None };
        // }
        // let obs_before = obs_before.unwrap();
        // if let Observation::NotSeen(disappeared) = obs_before {
        //     return Visibility::Invisible {
        //         disappeared: Some(*disappeared),
        //     };
        // }

        // // obs before is a Seen, so we're either Seen or Disappearing, depending on obs_after

        // if let Some(Observation::NotSeen(disappeared)) = obs_after {
        //     return Visibility::Disappearing {
        //         last_seen: obs_before.timestamp(),
        //         disappeared: *disappeared,
        //     };
        // }

        // // obs after either doesn't exist or it's a seen

        // if i == 0 {
        //     // all observations are later than timestamp or there are no observations at all
        //     return Visibility::Invisible { disappeared: None };
        // }
        // if i >= self.0.len() {
        //     // all observations are earlier than timestamp
        //     let l = self.0.len();
        //     let last_obs = self.0[l - 1];
        //     return match last_obs {
        //         Observation::Seen(t) => Visibility::Visible {
        //             first_seen: t,
        //             last_seen: None,
        //         },
        //         Observation::NotSeen(t) => Visibility::Invisible {
        //             disappeared: Some(t),
        //         },
        //     };
        // }

        // // timestamp is enclosed by two observations
        // let obs_before = self.0[i - 1];
        // let obs_after = self.0[i];
        // match obs_before {
        //     Observation::Seen(t_before) => match obs_after {
        //         Observation::Seen(t_after) => Visibility::Visible {
        //             first_seen: t_before,
        //             last_seen: Some(t_after),
        //         },
        //         Observation::NotSeen(t_after) => Visibility::Disappearing {
        //             last_seen: t_before,
        //             disappeared: t_after,
        //         },
        //     },
        //     Observation::NotSeen(t_before) => Visibility::Invisible {
        //         disappeared: Some(t_before),
        //     },
        // }
    }

    /// Append an observation, squashing the last observations if needed, i.e.,
    ///   - Add a NotSeen only if the last observation is a Seen.
    ///   - Don't add a Seen if the last two observations are Seens already. Instead, update the
    ///     timestamp of the last Seen.
    ///   - Add the observation in all other cases.
    ///
    /// Return an error if the timestamp of the new observation is earlier than the previous one.
    pub fn append(&mut self, o: Observation) -> Result<(), ChronologyError> {
        let num_obs = self.0.len();
        if num_obs == 0 {
            if let Observation::Seen(_) = o {
                self.0.push_back(o);
            }
            return Ok(());
        }

        let last_obs = self.0[num_obs - 1];
        if o.timestamp() < last_obs.timestamp() {
            return Err(ChronologyError {});
        }

        if let Observation::NotSeen(_) = o {
            if let Observation::Seen(_) = last_obs {
                self.0.push_back(o);
            }
        } else {
            // it's a Seen
            if num_obs <= 1 {
                self.0.push_back(o);
            } else if let Observation::NotSeen(_) = last_obs {
                self.0.push_back(o);
            } else if let Observation::NotSeen(_) = self.0[self.0.len() - 2] {
                self.0.push_back(o);
            } else {
                self.0[num_obs - 1] = o;
            }
        }
        Ok(())
    }

    /// Remove old and unnecessary observations assuming we're only interested in visibilities at or
    /// later than cutoff.
    ///
    /// The only information that might be lost is the timestamp in an early NotSeen.
    pub fn prune(&mut self, cutoff: Timestamp) {
        // keep the observation right before the cutoff, remove all earlier ones as they don't
        // affect visibility after the cutoff
        while self.0.len() >= 2 && self.0[1].timestamp() <= cutoff {
            self.0.pop_front();
        }
        // remove leading NotSeen observations (even if they happened after the cutoff) as they
        // carry no information
        while self.0.len() >= 1 && matches!(self.0[0], Observation::NotSeen(_)) {
            self.0.pop_front();
        }
    }
}

impl IntoIterator for Observations {
    type Item = Observation;
    type IntoIter = collections::vec_deque::IntoIter<Self::Item>;

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
            let obs = Observations::from_iter(test_case.obs).unwrap();
            let v = obs.visibility_at(test_case.t);
            assert_eq!(v, test_case.v);
        }
    }

    #[test]
    fn test_append_observation_error() {
        let mut obs = Observations::new();
        obs.append(seen(10)).unwrap();
        obs.append(seen(9)).unwrap_err();
    }

    #[test]
    fn test_append_observation_success() {
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
                o: seen(15),
                exp: vec![seen(10), seen(15)],
            },
            TestCase {
                obs: vec![seen(10)],
                o: not_seen(15),
                exp: vec![seen(10), not_seen(15)],
            },
            TestCase {
                obs: vec![seen(10), seen(15)],
                o: seen(20),
                exp: vec![seen(10), seen(20)],
            },
        ];
        for test_case in test_cases {
            let mut obs = Observations::from_iter(test_case.obs).unwrap();
            obs.append(test_case.o).unwrap();
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
            let mut obs = Observations::from_iter(test_case.obs.clone()).unwrap();
            obs.prune(test_case.t);
            let exp = test_case.obs[test_case.n_pruned..].into_iter().cloned();
            assert!(obs.into_iter().eq(exp));
        }
    }

    #[test]
    fn test_is_empty() {
        let empty = Observations::from_iter(vec![]).unwrap();
        assert!(empty.is_empty());
        let non_empty = Observations::from_iter(vec![Observation::Seen(10)]).unwrap();
        assert!(!non_empty.is_empty());
    }
}
