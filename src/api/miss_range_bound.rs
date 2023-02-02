use std::{str::FromStr, string::ToString};

use chrono::NaiveDateTime;
use thiserror::Error;

/// The start or end of a range of misses. It is given by a proposal time and,
/// to distinguish misses with equal proposal time, an offset that counts
/// misses. It is used both for requests and responses (to indicate the range
/// of results allowing follow up queries).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MissRangeBound {
    pub proposal_time: NaiveDateTime,
    pub offset: Option<usize>,
}

impl ToString for MissRangeBound {
    fn to_string(&self) -> String {
        let mut s = self.proposal_time.timestamp().to_string();
        if let Some(offset) = self.offset {
            s.push(',');
            s.push_str(offset.to_string().as_str());
        }
        s
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("empty string")]
    EmptyString,
    #[error("too many parts")]
    TooManyParts,
    #[error("invalid proposal timestamp")]
    InvalidProposalTimestamp,
    #[error("invalid offset")]
    InvalidOffset,
}

impl FromStr for MissRangeBound {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, ParseError> {
        if s.is_empty() {
            return Err(ParseError::EmptyString);
        }

        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() > 2 {
            return Err(ParseError::TooManyParts);
        }

        let proposal_time_int = parts[0]
            .parse()
            .map_err(|_| ParseError::InvalidProposalTimestamp)?;
        let proposal_time = NaiveDateTime::from_timestamp_opt(proposal_time_int, 0)
            .ok_or(ParseError::InvalidProposalTimestamp)?;

        let offset = parts
            .get(1)
            .map(|s| s.parse().map_err(|_| ParseError::InvalidOffset))
            .transpose()?;
        Ok(MissRangeBound {
            proposal_time,
            offset,
        })
    }
}

pub mod serde {
    pub mod miss_range_bound {
        use std::{fmt, str::FromStr};

        use chrono::NaiveDateTime;
        use serde::{de::Visitor, Deserializer, Serializer};

        use super::super::{MissRangeBound, ParseError};

        pub fn serialize<S>(t: &MissRangeBound, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if t.offset.is_some() {
                serializer.serialize_str(t.to_string().as_str())
            } else {
                serializer.serialize_i64(t.proposal_time.timestamp())
            }
        }

        #[allow(dead_code)]
        pub fn deserialize<'de, D>(d: D) -> Result<MissRangeBound, D::Error>
        where
            D: Deserializer<'de>,
        {
            d.deserialize_any(MissRangeBoundVisitor)
        }

        pub struct MissRangeBoundVisitor;

        impl<'de> Visitor<'de> for MissRangeBoundVisitor {
            type Value = MissRangeBound;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("integer or string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                MissRangeBound::from_str(v).map_err(serde::de::Error::custom)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let proposal_time = NaiveDateTime::from_timestamp_opt(v, 0).ok_or(
                    serde::de::Error::custom(ParseError::InvalidProposalTimestamp),
                )?;
                Ok(MissRangeBound {
                    proposal_time,
                    offset: None,
                })
            }
        }
    }

    pub mod miss_range_bound_option {
        use std::fmt;

        use serde::{de::Visitor, Deserializer, Serializer};

        use super::{super::MissRangeBound, miss_range_bound::MissRangeBoundVisitor};

        #[allow(dead_code)]
        pub fn serialize<S>(opt: &Option<MissRangeBound>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match opt {
                Some(t) => serializer.serialize_str(t.to_string().as_str()),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D>(d: D) -> Result<Option<MissRangeBound>, D::Error>
        where
            D: Deserializer<'de>,
        {
            d.deserialize_option(OptionMissRangeBoundVisitor)
        }

        struct OptionMissRangeBoundVisitor;

        impl<'de> Visitor<'de> for OptionMissRangeBoundVisitor {
            type Value = Option<MissRangeBound>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("integer, string, or none")
            }

            fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                d.deserialize_any(MissRangeBoundVisitor).map(Some)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(None)
            }
        }
    }
}
