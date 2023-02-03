use std::hash::Hash;

use actix_web::{web, Error, Result};
use chrono::{naive::serde::ts_seconds, NaiveDateTime};
use serde::Serialize;

use super::{
    miss_range_bound::MissRangeBound, requests::GroupedMissArgs, AppState, InternalError, MissArgs,
};

#[derive(Debug, Serialize, Clone)]
pub struct Miss {
    pub tx_hash: String,
    pub block_hash: String,
    pub slot: i32,
    pub block_number: i32,
    #[serde(with = "ts_seconds")]
    pub proposal_time: NaiveDateTime,
    pub proposer_index: i32,
    #[serde(with = "ts_seconds")]
    pub tx_first_seen: NaiveDateTime,
    #[serde(with = "ts_seconds")]
    pub tx_quorum_reached: NaiveDateTime,
    pub sender: String,
    pub tip: Option<i64>,
    #[serde(skip_serializing)]
    pub filtered_miss_count: i64,
    #[serde(skip_serializing)]
    pub filtered_miss_row_by_proposal_time: i64,
}

impl Hash for Miss {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.tx_hash.hash(state);
        self.block_hash.hash(state);
    }
}

impl PartialEq for Miss {
    fn eq(&self, other: &Self) -> bool {
        self.tx_hash == other.tx_hash && self.block_hash == other.block_hash
    }
}

impl Eq for Miss {}

pub async fn query_misses(args: &MissArgs, data: &web::Data<AppState>) -> Result<Vec<Miss>, Error> {
    let pool = &data.pool;
    let limit = data.config.api_max_response_rows;
    let (min, max) = args.checked_time_range(data.request_time)?;
    let result = sqlx::query_file_as!(
        Miss,
        "src/api/misses_query.sql",
        min,
        max,
        args.checked_block_number()?,
        args.checked_proposer_index()?,
        args.checked_sender()?,
        args.checked_propagation_time()?,
        args.checked_min_tip()?,
        args.checked_is_order_ascending(data.request_time)?,
        limit as i64,
        args.checked_from()?.offset.unwrap_or(0) as i64,
    )
    .fetch_all(pool)
    .await;

    match result {
        Err(e) => {
            log::error!("error fetching txs from db: {}", e);
            Err(Error::from(InternalError {}))
        }
        Ok(v) => Ok(v),
    }
}

pub async fn query_misses_for_txs(
    args: &GroupedMissArgs,
    data: &web::Data<AppState>,
) -> Result<Vec<Miss>, Error> {
    let pool = &data.pool;
    let limit = data.config.api_max_response_rows;
    let miss_args: MissArgs = args.clone().into();
    let (min, max) = miss_args.checked_time_range(data.request_time)?;
    let result = sqlx::query_file_as!(
        Miss,
        "src/api/txs_query.sql",
        min,
        max,
        miss_args.checked_block_number()?,
        miss_args.checked_proposer_index()?,
        miss_args.checked_sender()?,
        miss_args.checked_propagation_time()?,
        miss_args.checked_min_tip()?,
        miss_args.checked_is_order_ascending(data.request_time)?,
        limit as i64,
        miss_args.checked_from()?.offset.unwrap_or(0) as i64,
    )
    .fetch_all(pool)
    .await;

    match result {
        Err(e) => {
            log::error!("error fetching txs from db: {}", e);
            Err(Error::from(InternalError {}))
        }
        Ok(v) => Ok(v),
    }
}

pub async fn query_misses_for_blocks(
    args: &GroupedMissArgs,
    data: &web::Data<AppState>,
) -> Result<Vec<Miss>, Error> {
    let pool = &data.pool;
    let limit = data.config.api_max_response_rows;
    let miss_args: MissArgs = args.clone().into();
    let (min, max) = miss_args.checked_time_range(data.request_time)?;
    let result = sqlx::query_file_as!(
        Miss,
        "src/api/blocks_query.sql",
        min,
        max,
        miss_args.checked_block_number()?,
        miss_args.checked_proposer_index()?,
        miss_args.checked_sender()?,
        miss_args.checked_propagation_time()?,
        miss_args.checked_min_tip()?,
        miss_args.checked_is_order_ascending(data.request_time)?,
        limit as i64,
        miss_args.checked_from()?.offset.unwrap_or(0) as i64,
    )
    .fetch_all(pool)
    .await;

    match result {
        Err(e) => {
            log::error!("error fetching txs from db: {}", e);
            Err(Error::from(InternalError {}))
        }
        Ok(v) => Ok(v),
    }
}

pub fn is_query_complete(misses: &[Miss], limit: usize) -> bool {
    let filtered_miss_count = misses
        .get(0)
        .map(|miss| miss.filtered_miss_count)
        .unwrap_or(0);
    filtered_miss_count < limit as i64
}

pub fn get_end_bound(misses: &[Miss], query_from: &MissRangeBound) -> Option<MissRangeBound> {
    misses.last().map(|last_miss| {
        let offset_inclusive = (last_miss.filtered_miss_row_by_proposal_time as usize)
            + if last_miss.proposal_time == query_from.proposal_time {
                query_from.offset.unwrap_or(0)
            } else {
                0
            };
        MissRangeBound {
            proposal_time: last_miss.proposal_time,
            offset: Some(offset_inclusive + 1),
        }
    })
}
