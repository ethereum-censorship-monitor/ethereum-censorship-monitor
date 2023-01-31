use std::{str::FromStr, string::ToString};

use chrono::NaiveDateTime;
use thiserror::Error;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MissTimeTuple {
    pub proposal_time: NaiveDateTime,
    pub tx_quorum_reached: Option<NaiveDateTime>,
}

impl ToString for MissTimeTuple {
    fn to_string(&self) -> String {
        let mut s = self.proposal_time.timestamp().to_string();
        if let Some(tx_quorum_reached) = self.tx_quorum_reached {
            s.push(',');
            s.push_str(tx_quorum_reached.timestamp().to_string().as_str());
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
    #[error("invalid tx quorum reached timestamp")]
    InvalidTxQuorumReachedTimestamp,
}

impl FromStr for MissTimeTuple {
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

        let tx_quorum_reached = if let Some(tx_quorum_reached_str) = parts.get(1) {
            let tx_quorum_reached_int = tx_quorum_reached_str
                .parse()
                .map_err(|_| ParseError::InvalidTxQuorumReachedTimestamp)?;
            Some(
                NaiveDateTime::from_timestamp_opt(tx_quorum_reached_int, 0)
                    .ok_or(ParseError::InvalidTxQuorumReachedTimestamp)?,
            )
        } else {
            None
        };

        Ok(MissTimeTuple {
            proposal_time,
            tx_quorum_reached,
        })
    }
}

pub mod serde_miss_time_tuple {
    use std::{fmt, str::FromStr};

    use serde::{de::Visitor, Deserializer, Serializer};

    use super::MissTimeTuple;

    pub fn serialize<S>(t: &MissTimeTuple, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(t.to_string().as_str())
    }

    #[allow(dead_code)]
    pub fn deserialize<'de, D>(d: D) -> Result<MissTimeTuple, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_str(MissTimeTupleVisitor)
    }

    pub struct MissTimeTupleVisitor;

    impl<'de> Visitor<'de> for MissTimeTupleVisitor {
        type Value = MissTimeTuple;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or none")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            MissTimeTuple::from_str(v).map_err(serde::de::Error::custom)
        }
    }
}

pub mod serde_opt_miss_time_tuple {
    use std::fmt;

    use serde::{de::Visitor, Deserializer, Serializer};

    use super::{serde_miss_time_tuple::MissTimeTupleVisitor, MissTimeTuple};

    #[allow(dead_code)]
    pub fn serialize<S>(opt: &Option<MissTimeTuple>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match opt {
            Some(t) => serializer.serialize_str(t.to_string().as_str()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<MissTimeTuple>, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_option(OptionMissTimeTupleVisitor)
    }

    struct OptionMissTimeTupleVisitor;

    impl<'de> Visitor<'de> for OptionMissTimeTupleVisitor {
        type Value = Option<MissTimeTuple>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or none")
        }

        fn visit_some<D>(self, d: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            d.deserialize_str(MissTimeTupleVisitor).map(Some)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }
}
