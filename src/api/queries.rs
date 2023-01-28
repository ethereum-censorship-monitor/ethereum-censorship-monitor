use actix_web::{web, Error, Result};
use chrono::{naive::serde::ts_seconds, NaiveDateTime};
use serde::Serialize;
use sqlx::types::JsonValue;

use super::{requests::GroupedMissArgs, AppState, InternalError, MissArgs, ResponseItem};

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
    pub ref_time: NaiveDateTime,
}

pub async fn query_misses(args: &MissArgs, data: &web::Data<AppState>) -> Result<Vec<Miss>, Error> {
    let pool = &data.pool;
    let limit = data.config.api_max_response_rows;
    let result = sqlx::query_file_as!(
        Miss,
        "src/api/misses_query.sql",
        args.checked_time_range(data.request_time)?.0,
        args.checked_time_range(data.request_time)?.1,
        args.checked_block_number()?,
        args.checked_proposer_index()?,
        args.checked_sender()?,
        args.checked_propagation_time()?,
        args.checked_min_tip()?,
        args.checked_is_order_ascending(data.request_time)?,
        (limit + 1) as i64,
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

#[derive(Debug, Serialize, Clone)]
pub struct Tx {
    pub tx_hash: String,
    #[serde(with = "ts_seconds")]
    pub tx_first_seen: NaiveDateTime,
    #[serde(with = "ts_seconds")]
    pub tx_quorum_reached: NaiveDateTime,
    pub sender: String,
    pub blocks: JsonValue,
    pub num_misses: i64,
    #[serde(skip_serializing)]
    pub ref_time: NaiveDateTime,
}

pub async fn query_txs(
    args: &GroupedMissArgs,
    data: &web::Data<AppState>,
) -> Result<Vec<Tx>, Error> {
    let pool = &data.pool;
    let limit = data.config.api_max_response_rows;
    let miss_args: MissArgs = args.clone().into();
    let result = sqlx::query_file_as!(
        Tx,
        "src/api/txs_query.sql",
        miss_args.checked_time_range(data.request_time)?.0,
        miss_args.checked_time_range(data.request_time)?.1,
        miss_args.checked_block_number()?,
        miss_args.checked_proposer_index()?,
        miss_args.checked_sender()?,
        miss_args.checked_propagation_time()?,
        miss_args.checked_min_tip()?,
        miss_args.checked_is_order_ascending(data.request_time)?,
        (limit + 1) as i64,
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

#[derive(Debug, Serialize, Clone)]
pub struct Block {
    pub block_hash: String,
    pub slot: i32,
    pub block_number: i32,
    #[serde(with = "ts_seconds")]
    pub proposal_time: NaiveDateTime,
    pub proposer_index: i32,
    pub num_misses: i64,
    pub txs: JsonValue,
    #[serde(skip_serializing)]
    pub ref_time: NaiveDateTime,
    #[serde(skip_serializing)]
    pub ref_row_number: i64,
}

pub async fn query_blocks(
    args: &GroupedMissArgs,
    data: &web::Data<AppState>,
) -> Result<Vec<Block>, Error> {
    let pool = &data.pool;
    let limit = data.config.api_max_response_rows;
    let miss_args: MissArgs = args.clone().into();
    let result = sqlx::query_file_as!(
        Block,
        "src/api/blocks_query.sql",
        miss_args.checked_time_range(data.request_time)?.0,
        miss_args.checked_time_range(data.request_time)?.1,
        miss_args.checked_block_number()?,
        miss_args.checked_proposer_index()?,
        miss_args.checked_sender()?,
        miss_args.checked_propagation_time()?,
        miss_args.checked_min_tip()?,
        miss_args.checked_is_order_ascending(data.request_time)?,
        (limit + 1) as i64,
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

impl ResponseItem for Miss {
    fn get_ref_time(&self) -> NaiveDateTime {
        self.ref_time
    }
}

impl ResponseItem for Tx {
    fn get_ref_time(&self) -> NaiveDateTime {
        self.ref_time
    }
}

impl ResponseItem for Block {
    fn get_ref_time(&self) -> NaiveDateTime {
        self.ref_time
    }
}
