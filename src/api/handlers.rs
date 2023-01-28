use actix_web::{
    get,
    web::{self, Json, Query},
    Error, Responder, Result,
};

use super::{
    get_last_ref_time, query_blocks, query_misses, query_txs, AppState, GroupedMissArgs,
    ItemizedResponse, MissArgs,
};

#[get("/v0/misses")]
pub async fn handle_misses(
    data: web::Data<AppState>,
    q: Query<MissArgs>,
) -> Result<impl Responder, Error> {
    let misses = query_misses(&q.0, &data).await?;
    let complete = misses.len() <= data.config.api_max_response_rows;
    let last_time = get_last_ref_time(&misses);
    let response = ItemizedResponse::new(
        misses,
        complete,
        q.0.checked_from()?,
        q.0.checked_to(data.request_time)?,
        last_time,
    );
    Ok(Json(response))
}

#[get("/v0/txs")]
pub async fn handle_txs(
    data: web::Data<AppState>,
    q: Query<GroupedMissArgs>,
) -> Result<impl Responder, Error> {
    let txs = query_txs(&q.0, &data).await?;
    let complete = txs.len() <= data.config.api_max_response_rows;
    let last_time = get_last_ref_time(&txs);
    let min_num_misses = q.checked_min_num_misses()?;
    let filtered_txs = txs
        .iter()
        .filter(|tx| min_num_misses.is_none() || tx.num_misses >= min_num_misses.unwrap())
        .cloned()
        .collect();
    let miss_args: MissArgs = q.0.into();
    let response = ItemizedResponse::new(
        filtered_txs,
        complete,
        miss_args.checked_from()?,
        miss_args.checked_to(data.request_time)?,
        last_time,
    );
    Ok(Json(response))
}

#[get("/v0/blocks")]
pub async fn handle_blocks(
    data: web::Data<AppState>,
    q: Query<GroupedMissArgs>,
) -> Result<impl Responder, Error> {
    let mut blocks = query_blocks(&q.0, &data).await?;
    let num_original_rows = blocks.iter().map(|b| b.ref_row_number).max().unwrap_or(0) as usize;
    let complete = if num_original_rows <= data.config.api_max_response_rows {
        true
    } else {
        blocks.retain(|b| (b.ref_row_number as usize) < num_original_rows);
        false
    };
    let last_time = get_last_ref_time(&blocks);
    let min_num_misses = q.checked_min_num_misses()?;
    let filtered_blocks = blocks
        .iter()
        .filter(|block| min_num_misses.is_none() || block.num_misses >= min_num_misses.unwrap())
        .cloned()
        .collect();
    let miss_args: MissArgs = q.0.into();
    let response = ItemizedResponse::new(
        filtered_blocks,
        complete,
        miss_args.checked_from()?,
        miss_args.checked_to(data.request_time)?,
        last_time,
    );
    Ok(Json(response))
}
