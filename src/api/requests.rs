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
    pub fn checked_from(&self) -> Result<NaiveDateTime, RequestError> {
        let t = from_opt_timestamp(self.from, String::from("from"))?;
        Ok(t.unwrap_or_else(|| NaiveDateTime::from_timestamp_opt(0, 0).unwrap()))
    }

    pub fn checked_to(&self, request_time: NaiveDateTime) -> Result<NaiveDateTime, RequestError> {
        let t = from_opt_timestamp(self.to, String::from("to"))?;
        Ok(t.unwrap_or(request_time))
    }

    pub fn checked_time_range(
        &self,
        request_time: NaiveDateTime,
    ) -> Result<(NaiveDateTime, NaiveDateTime), RequestError> {
        let from = self.checked_from()?;
        let to = self.checked_to(request_time)?;
        Ok((min(from, to), max(from, to)))
    }

    pub fn checked_is_order_ascending(
        &self,
        request_time: NaiveDateTime,
    ) -> Result<bool, RequestError> {
        Ok(self.checked_from()? <= self.checked_to(request_time)?)
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
