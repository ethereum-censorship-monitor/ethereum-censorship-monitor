use actix_web::{
    get,
    web::{self, Json, Query},
    Error, Responder, Result,
};

use super::{
    get_time_range, query_blocks, query_misses, query_txs, AppState, GroupedMissArgs,
    ItemizedResponse, MissArgs,
};

#[get("/v0/misses")]
pub async fn handle_misses(
    data: web::Data<AppState>,
    q: Query<MissArgs>,
) -> Result<impl Responder, Error> {
    let limit = data.config.api_max_response_rows;
    let misses = query_misses(&q.0, &data.pool, limit).await?;
    let time_range = get_time_range(&misses);
    let response =
        ItemizedResponse::new(misses, time_range, q.0.checked_from()?, q.0.checked_to()?);
    Ok(Json(response))
}

#[get("/v0/txs")]
pub async fn handle_txs(
    data: web::Data<AppState>,
    q: Query<GroupedMissArgs>,
) -> Result<impl Responder, Error> {
    let limit = data.config.api_max_response_rows;
    let min_num_misses = q.checked_min_num_misses()?;

    let txs = query_txs(&q.0, &data.pool, limit).await?;
    let time_range = get_time_range(&txs);
    let filtered_txs = txs
        .iter()
        .filter(|tx| min_num_misses.is_none() || tx.num_misses >= min_num_misses.unwrap())
        .cloned()
        .collect();
    let miss_args: MissArgs = q.0.into();
    let response = ItemizedResponse::new(
        filtered_txs,
        time_range,
        miss_args.checked_from()?,
        miss_args.checked_to()?,
    );
    Ok(Json(response))
}

#[get("/v0/blocks")]
pub async fn handle_blocks(
    data: web::Data<AppState>,
    q: Query<GroupedMissArgs>,
) -> Result<impl Responder, Error> {
    let limit = data.config.api_max_response_rows;
    let min_num_misses = q.checked_min_num_misses()?;

    let blocks = query_blocks(&q.0, &data.pool, limit).await?;
    let time_range = get_time_range(&blocks);
    let filtered_blocks = blocks
        .iter()
        .filter(|block| min_num_misses.is_none() || block.num_misses >= min_num_misses.unwrap())
        .cloned()
        .collect();
    let miss_args: MissArgs = q.0.into();
    let response = ItemizedResponse::new(
        filtered_blocks,
        time_range,
        miss_args.checked_from()?,
        miss_args.checked_to()?,
    );
    Ok(Json(response))
}
