use std::cmp::{max, min};

use actix_web::Result;
use chrono::{Duration, NaiveDateTime};
use serde::Deserialize;
use sqlx::postgres::types::PgInterval;

use super::RequestError;

#[derive(Deserialize, Clone)]
pub struct MissArgs {
    from: Option<i64>,
    to: Option<i64>,
    block_number: Option<i32>,
    proposer_index: Option<i32>,
    sender: Option<String>,
    propagation_time: Option<i64>,
    min_tip: Option<i64>,
}

#[derive(Deserialize, Clone)]
pub struct GroupedMissArgs {
    from: Option<i64>,
    to: Option<i64>,
    block_number: Option<i32>,
    proposer_index: Option<i32>,
    sender: Option<String>,
    propagation_time: Option<i64>,
    min_tip: Option<i64>,
    min_num_misses: Option<i64>,
}

impl MissArgs {
    pub fn checked_from(&self) -> Result<Option<NaiveDateTime>, RequestError> {
        from_opt_timestamp(self.from, String::from("from"))
    }

    pub fn checked_to(&self) -> Result<Option<NaiveDateTime>, RequestError> {
        from_opt_timestamp(self.to, String::from("to"))
    }

    pub fn checked_time_range(
        &self,
    ) -> Result<(Option<NaiveDateTime>, Option<NaiveDateTime>), RequestError> {
        Ok(ordered_timestamps(self.checked_from()?, self.checked_to()?))
    }

    pub fn checked_is_order_ascending(&self) -> Result<bool, RequestError> {
        Ok(is_order_ascending(self.checked_from()?, self.checked_to()?))
    }

    pub fn checked_block_number(&self) -> Result<Option<i32>, RequestError> {
        from_opt_nonneg_uint(self.block_number, String::from("block_number"))
    }

    pub fn checked_proposer_index(&self) -> Result<Option<i32>, RequestError> {
        from_opt_nonneg_uint(self.proposer_index, String::from("proposer_index"))
    }

    pub fn checked_sender(&self) -> Result<Option<&String>, RequestError> {
        Ok(self.sender.as_ref())
    }

    pub fn checked_propagation_time(&self) -> Result<Option<PgInterval>, RequestError> {
        from_opt_interval(self.propagation_time, String::from("propagation_time"))
    }

    pub fn checked_min_tip(&self) -> Result<Option<i64>, RequestError> {
        from_opt_nonneg_uint(self.min_tip, String::from("min_tip"))
    }
}

impl GroupedMissArgs {
    pub fn checked_min_num_misses(&self) -> Result<Option<i64>, RequestError> {
        from_opt_nonneg_uint(self.min_num_misses, String::from("min_num_misses"))
    }
}

impl From<GroupedMissArgs> for MissArgs {
    fn from(m: GroupedMissArgs) -> Self {
        Self {
            from: m.from,
            to: m.to,
            block_number: m.block_number,
            proposer_index: m.proposer_index,
            sender: m.sender,
            propagation_time: m.propagation_time,
            min_tip: m.min_tip,
        }
    }
}

fn from_opt_timestamp(
    i: Option<i64>,
    parameter: String,
) -> Result<Option<NaiveDateTime>, RequestError> {
    if i.is_none() {
        return Ok(None);
    }
    let i = i.unwrap();

    if i < 0 {
        return Err(RequestError::ParameterOutOfRange { parameter });
    }
    let t = NaiveDateTime::from_timestamp_opt(i, 0);
    if t.is_none() {
        Err(RequestError::ParameterOutOfRange { parameter })
    } else {
        Ok(t)
    }
}

fn from_opt_interval(
    i: Option<i64>,
    parameter: String,
) -> Result<Option<PgInterval>, RequestError> {
    if i.is_none() {
        return Ok(None);
    }
    let i = i.unwrap();

    if i < 0 {
        return Err(RequestError::ParameterOutOfRange { parameter });
    }
    let duration = Duration::seconds(i);
    let interval =
        PgInterval::try_from(duration).map_err(|_| RequestError::ParameterOutOfRange {
            parameter: String::from("propagation_time"),
        })?;
    Ok(Some(interval))
}

fn from_opt_nonneg_uint<T>(i: Option<T>, parameter: String) -> Result<Option<T>, RequestError>
where
    T: Into<i64>,
    T: Clone,
{
    if i.is_none() {
        return Ok(i);
    }
    let n: i64 = i.clone().unwrap().into();
    if n < 0 {
        return Err(RequestError::ParameterOutOfRange { parameter });
    }
    Ok(i)
}

fn ordered_timestamps(
    from: Option<NaiveDateTime>,
    to: Option<NaiveDateTime>,
) -> (Option<NaiveDateTime>, Option<NaiveDateTime>) {
    match (from, to) {
        (None, _) => (from, to),
        (_, None) => (from, to),
        (Some(from), Some(to)) => (Some(min(from, to)), Some(max(from, to))),
    }
}

fn is_order_ascending(t1: Option<NaiveDateTime>, t2: Option<NaiveDateTime>) -> bool {
    match (t1, t2) {
        (None, _) => true,
        (_, None) => true,
        (Some(from), Some(to)) => from <= to,
    }
}
