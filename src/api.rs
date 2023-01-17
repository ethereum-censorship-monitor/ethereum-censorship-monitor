use std::{
    cmp::{max, min},
    collections::HashMap,
};

use actix_web::{
    get,
    http::StatusCode,
    web::{self, Json, Query},
    App, Error, HttpServer, Responder, ResponseError, Result,
};
use chrono::{
    naive::serde::{ts_seconds, ts_seconds_option},
    Duration, NaiveDateTime,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::PgInterval;
use thiserror::Error;

use crate::{cli::Config, db};

struct AppState {
    config: Config,
    pool: db::Pool,
}

pub async fn serve_api(config: Config) -> Result<(), std::io::Error> {
    let pool = db::connect(&config.api_db_connection).await.unwrap();
    let host_and_port = (config.api_host.clone(), config.api_port);
    HttpServer::new(move || {
        let state = AppState {
            config: config.clone(),
            pool: pool.clone(),
        };
        App::new()
            .app_data(web::Data::new(state))
            .service(misses_handler)
            .service(txs_handler)
            .service(blocks_handler)
    })
    .bind(host_and_port)?
    .run()
    .await
}

#[get("/v0/misses")]
async fn misses_handler(
    data: web::Data<AppState>,
    q: Query<MissArgs>,
) -> Result<impl Responder, Error> {
    let limit = data.config.api_max_response_rows;
    let misses = query_misses(&q.0, &data.pool, limit).await?;
    let miss_range = get_miss_range(&misses);
    let response =
        ItemizedResponse::new(misses, miss_range, q.0.checked_from()?, q.0.checked_to()?);
    Ok(Json(response))
}

#[get("/v0/txs")]
async fn txs_handler(
    data: web::Data<AppState>,
    q: Query<GroupedMissArgs>,
) -> Result<impl Responder, Error> {
    let limit = data.config.api_max_response_rows;
    let min_num_misses = &q.0.checked_min_num_misses()?;
    let misses_args: &MissArgs = &q.0.into();

    let misses = query_misses(misses_args, &data.pool, limit).await?;
    let mut txs: Vec<Tx> = group_by_tx(&misses)
        .values()
        .filter(|tx| min_num_misses.is_none() || tx.num_misses >= min_num_misses.unwrap())
        .cloned()
        .collect();
    txs.sort_by_key(|tx| tx.blocks[0].proposal_time);

    let miss_range = get_miss_range(&misses);
    let response = ItemizedResponse::new(
        txs,
        miss_range,
        misses_args.checked_from()?,
        misses_args.checked_to()?,
    );
    Ok(Json(response))
}

fn group_by_tx(misses: &[Miss]) -> HashMap<&String, Tx> {
    misses.iter().fold(HashMap::new(), |mut acc, miss| {
        let mut tx = acc.entry(&miss.tx_hash).or_insert(Tx {
            tx_hash: miss.tx_hash.clone(),
            tx_first_seen: miss.tx_first_seen,
            tx_quorum_reached: miss.tx_quorum_reached,
            sender: miss.sender.clone(),
            num_misses: 0,
            blocks: Vec::new(),
        });
        tx.num_misses += 1;
        tx.blocks.push(MissWithoutTxFields {
            block_hash: miss.block_hash.clone(),
            slot: miss.slot,
            block_number: miss.block_number,
            proposal_time: miss.proposal_time,
            proposer_index: miss.proposer_index,
            tip: miss.tip,
        });
        acc
    })
}

#[get("/v0/blocks")]
async fn blocks_handler(
    data: web::Data<AppState>,
    q: Query<GroupedMissArgs>,
) -> Result<impl Responder, Error> {
    let limit = data.config.api_max_response_rows;
    let min_num_misses = &q.0.checked_min_num_misses()?;
    let misses_args: &MissArgs = &q.0.into();

    let misses = query_misses(misses_args, &data.pool, limit).await?;
    let mut blocks: Vec<Block> = group_by_block(&misses)
        .values()
        .filter(|block| min_num_misses.is_none() || block.num_misses >= min_num_misses.unwrap())
        .cloned()
        .collect();
    blocks.sort_by_key(|block| block.proposal_time);

    let miss_range = get_miss_range(&misses);
    let response = ItemizedResponse::new(
        blocks,
        miss_range,
        misses_args.checked_from()?,
        misses_args.checked_to()?,
    );
    Ok(Json(response))
}

fn group_by_block(misses: &[Miss]) -> HashMap<&String, Block> {
    misses.iter().fold(HashMap::new(), |mut acc, miss| {
        let mut block = acc.entry(&miss.block_hash).or_insert(Block {
            block_hash: miss.block_hash.clone(),
            slot: miss.slot,
            block_number: miss.block_number,
            proposal_time: miss.proposal_time,
            proposer_index: miss.proposer_index,
            num_misses: 0,
            txs: Vec::new(),
        });
        block.num_misses += 1;
        block.txs.push(MissWithoutBlockFields {
            tx_hash: miss.tx_hash.clone(),
            tx_first_seen: miss.tx_first_seen,
            tx_quorum_reached: miss.tx_quorum_reached,
            sender: miss.sender.clone(),
            tip: miss.tip,
        });
        acc
    })
}

#[derive(Deserialize)]
struct MissArgs {
    from: Option<i64>,
    to: Option<i64>,
    block_number: Option<i32>,
    proposer_index: Option<i32>,
    sender: Option<String>,
    propagation_time: Option<i64>,
    min_tip: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
struct Miss {
    tx_hash: String,
    block_hash: String,
    slot: i32,
    block_number: i32,
    #[serde(with = "ts_seconds")]
    proposal_time: NaiveDateTime,
    proposer_index: i32,
    #[serde(with = "ts_seconds")]
    tx_first_seen: NaiveDateTime,
    #[serde(with = "ts_seconds")]
    tx_quorum_reached: NaiveDateTime,
    sender: String,
    tip: Option<i64>,
}

#[derive(Deserialize)]
struct GroupedMissArgs {
    from: Option<i64>,
    to: Option<i64>,
    block_number: Option<i32>,
    proposer_index: Option<i32>,
    sender: Option<String>,
    propagation_time: Option<i64>,
    min_tip: Option<i64>,
    min_num_misses: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
struct Tx {
    tx_hash: String,
    #[serde(with = "ts_seconds")]
    tx_first_seen: NaiveDateTime,
    #[serde(with = "ts_seconds")]
    tx_quorum_reached: NaiveDateTime,
    sender: String,
    num_misses: i64,
    blocks: Vec<MissWithoutTxFields>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MissWithoutTxFields {
    block_hash: String,
    slot: i32,
    block_number: i32,
    #[serde(with = "ts_seconds")]
    proposal_time: NaiveDateTime,
    proposer_index: i32,
    tip: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
struct Block {
    block_hash: String,
    slot: i32,
    block_number: i32,
    proposal_time: NaiveDateTime,
    proposer_index: i32,
    num_misses: i64,
    txs: Vec<MissWithoutBlockFields>,
}

#[derive(Debug, Serialize, Clone)]
struct MissWithoutBlockFields {
    tx_hash: String,
    tx_first_seen: NaiveDateTime,
    tx_quorum_reached: NaiveDateTime,
    sender: String,
    tip: Option<i64>,
}

impl MissArgs {
    fn checked_from(&self) -> Result<Option<NaiveDateTime>, RequestError> {
        from_opt_timestamp(self.from, String::from("from"))
    }

    fn checked_to(&self) -> Result<Option<NaiveDateTime>, RequestError> {
        from_opt_timestamp(self.to, String::from("to"))
    }

    fn checked_time_range(
        &self,
    ) -> Result<(Option<NaiveDateTime>, Option<NaiveDateTime>), RequestError> {
        Ok(ordered_timestamps(self.checked_from()?, self.checked_to()?))
    }

    fn checked_is_order_ascending(&self) -> Result<bool, RequestError> {
        Ok(is_order_ascending(self.checked_from()?, self.checked_to()?))
    }

    fn checked_block_number(&self) -> Result<Option<i32>, RequestError> {
        from_opt_nonneg_uint(self.block_number, String::from("block_number"))
    }

    fn checked_proposer_index(&self) -> Result<Option<i32>, RequestError> {
        from_opt_nonneg_uint(self.proposer_index, String::from("proposer_index"))
    }

    fn checked_sender(&self) -> Result<Option<&String>, RequestError> {
        Ok(self.sender.as_ref())
    }

    fn checked_propagation_time(&self) -> Result<Option<PgInterval>, RequestError> {
        from_opt_interval(self.propagation_time, String::from("propagation_time"))
    }

    fn checked_min_tip(&self) -> Result<Option<i64>, RequestError> {
        from_opt_nonneg_uint(self.min_tip, String::from("min_tip"))
    }
}

impl GroupedMissArgs {
    fn checked_min_num_misses(&self) -> Result<Option<i64>, RequestError> {
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

#[derive(Debug, Error)]
#[error("internal error")]
pub struct InternalError {}

impl ResponseError for InternalError {}

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("Query parameter {parameter} is out of range")]
    ParameterOutOfRange { parameter: String },
}

impl ResponseError for RequestError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

#[derive(Debug, Serialize)]
pub struct ItemizedResponse<T> {
    items: Vec<T>,
    #[serde(with = "ts_seconds_option")]
    from: Option<NaiveDateTime>,
    #[serde(with = "ts_seconds_option")]
    to: Option<NaiveDateTime>,
}

impl<T> ItemizedResponse<T> {
    fn new(
        items: Vec<T>,
        data_range: Option<(NaiveDateTime, NaiveDateTime)>,
        query_from: Option<NaiveDateTime>,
        query_to: Option<NaiveDateTime>,
    ) -> Self {
        let (from, to) = if let Some((from, to)) = data_range {
            (Some(from), Some(to))
        } else {
            (query_from, query_to)
        };
        Self { items, from, to }
    }
}

async fn query_misses(args: &MissArgs, pool: &db::Pool, limit: usize) -> Result<Vec<Miss>, Error> {
    let result = sqlx::query_file_as!(
        Miss,
        "src/misses_query.sql",
        args.checked_time_range()?.0,
        args.checked_time_range()?.1,
        args.checked_block_number()?,
        args.checked_proposer_index()?,
        args.checked_sender()?,
        args.checked_propagation_time()?,
        args.checked_min_tip()?,
        args.checked_is_order_ascending()?,
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

fn get_miss_range(misses: &Vec<Miss>) -> Option<(NaiveDateTime, NaiveDateTime)> {
    if misses.is_empty() {
        None
    } else {
        Some((
            misses[0].proposal_time,
            misses[misses.len() - 1].proposal_time,
        ))
    }
}
