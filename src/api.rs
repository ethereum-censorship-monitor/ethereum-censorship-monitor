use actix_web::{
    get,
    http::StatusCode,
    web::{self, Json, Query},
    App, Error, HttpServer, Responder, ResponseError, Result,
};
use chrono::{naive::serde::ts_seconds, Duration, NaiveDateTime};
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
        App::new().app_data(web::Data::new(state)).service(misses)
        // .service(txs)
        // .service(blocks)
    })
    .bind(host_and_port)?
    .run()
    .await
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
pub struct Miss {
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
struct MissesArgs {
    from: Option<i64>,
    to: Option<i64>,
    block_number: Option<i32>,
    proposer_index: Option<i32>,
    sender: Option<String>,
    propagation_time: Option<i64>,
    min_tip: Option<i64>,
}

#[get("/v0/misses")]
async fn misses(data: web::Data<AppState>, q: Query<MissesArgs>) -> Result<impl Responder, Error> {
    let from = from_opt_timestamp(q.from, String::from("from"))?;
    let to = from_opt_timestamp(q.to, String::from("to"))?;

    let block_number = from_opt_nonneg_uint(q.block_number, String::from("block_number"))?;
    let proposer_index = from_opt_nonneg_uint(q.proposer_index, String::from("proposer_index"))?;
    let sender = q.sender.clone();

    let propagation_time = from_opt_interval(q.propagation_time, String::from("propagation_time"))?;

    let min_tip = from_opt_nonneg_uint(q.min_tip, String::from("min_tip"))?;

    let limit = data.config.api_max_response_rows;

    let result = sqlx::query_as!(
        Miss,
        r#"
        SELECT
            tx_hash,
            block_hash,
            slot,
            block_number,
            proposal_time,
            proposer_index,
            tx_first_seen,
            tx_quorum_reached,
            sender,
            tip
        FROM
            data.full_miss
        WHERE
            ($1::timestamp IS NULL OR proposal_time > $1) AND 
            ($2::timestamp IS NULL OR proposal_time <= $2) AND
            ($3::integer IS NULL OR block_number = $3) AND
            ($4::integer IS NULL OR proposer_index = $4) AND
            ($5::char(42) IS NULL OR sender = $5) AND
            ($6::interval IS NULL OR proposal_time - tx_quorum_reached > $6) AND
            ($7::bigint IS NULL OR tip >= $7)
        LIMIT $8
        "#,
        from,
        to,
        block_number,
        proposer_index,
        sender,
        propagation_time,
        min_tip,
        limit as i64,
    )
    .fetch_all(&data.pool)
    .await;

    match result {
        Ok(misses) => Ok(Json(misses)),
        Err(e) => {
            log::error!("error fetching misses from db: {}", e);
            Err(Error::from(InternalError {}))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Tx {
    tx_hash: String,
    block_hash: String,
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

// #[derive(Deserialize)]
// struct TxsArgs {}

// #[get("/v0/txs")]
// async fn txs(q: web::Query<TxsArgs>) -> impl Responder {
//     HttpResponse::Ok().body("todo")
// }

// #[derive(Deserialize)]
// struct BlocksArgs {}

// #[get("/v0/blocks")]
// async fn blocks(q: web::Query<BlocksArgs>) -> impl Responder {
//     HttpResponse::Ok().body("todo")
// }

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
