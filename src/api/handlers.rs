use actix_web::{
    get,
    web::{self, Json, Query},
    Error, Responder, Result,
};
use itertools::Itertools;

use super::{
    get_end_bound, group_misses_to_blocks, group_misses_to_txs, is_query_complete, query_misses,
    query_misses_for_blocks, query_misses_for_txs, AppState, Block, GroupedMissArgs,
    ItemizedResponse, Miss, MissArgs, Tx,
};

#[get("/v0/misses")]
pub async fn handle_misses(
    data: web::Data<AppState>,
    q: Query<MissArgs>,
) -> Result<impl Responder, Error> {
    let misses = query_misses(&q.0, &data).await?;

    let complete = is_query_complete(&misses, data.config.api_max_response_rows);
    let data_to = get_end_bound(&misses, &q.0.checked_from()?);

    let response = ItemizedResponse::new(
        misses,
        complete,
        q.0.checked_from()?,
        q.0.checked_to(data.request_time)?,
        data_to,
    );
    Ok(Json(response))
}

#[get("/v0/txs")]
pub async fn handle_txs(
    data: web::Data<AppState>,
    q: Query<GroupedMissArgs>,
) -> Result<impl Responder, Error> {
    let misses = query_misses_for_txs(&q.0, &data).await?;
    let misses: Vec<Miss> = misses.into_iter().unique().collect();

    let min_num_misses = q.checked_min_num_misses()?;
    let miss_args: MissArgs = q.0.into();

    let complete = is_query_complete(&misses, data.config.api_max_response_rows);
    let data_to = get_end_bound(&misses, &miss_args.checked_from()?);

    let mut txs: Vec<Tx> = group_misses_to_txs(&misses)
        .iter()
        .filter(|tx| min_num_misses.is_none() || tx.num_misses as i64 >= min_num_misses.unwrap())
        .cloned()
        .collect();
    txs.sort();
    if !miss_args.checked_is_order_ascending(data.request_time)? {
        txs.reverse();
    }

    let response = ItemizedResponse::new(
        txs,
        complete,
        miss_args.checked_from()?,
        miss_args.checked_to(data.request_time)?,
        data_to,
    );
    Ok(Json(response))
}

#[get("/v0/blocks")]
pub async fn handle_blocks(
    data: web::Data<AppState>,
    q: Query<GroupedMissArgs>,
) -> Result<impl Responder, Error> {
    let misses = query_misses_for_blocks(&q.0, &data).await?;
    let misses: Vec<Miss> = misses.into_iter().unique().collect();

    let min_num_misses = q.checked_min_num_misses()?;
    let miss_args: MissArgs = q.0.into();

    let complete = is_query_complete(&misses, data.config.api_max_response_rows);
    let data_to = get_end_bound(&misses, &miss_args.checked_from()?);

    let mut blocks: Vec<Block> = group_misses_to_blocks(&misses)
        .iter()
        .filter(|block| {
            min_num_misses.is_none() || block.num_misses as i64 >= min_num_misses.unwrap()
        })
        .cloned()
        .collect();
    blocks.sort();
    if !miss_args.checked_is_order_ascending(data.request_time)? {
        blocks.reverse();
    }

    let response = ItemizedResponse::new(
        blocks,
        complete,
        miss_args.checked_from()?,
        miss_args.checked_to(data.request_time)?,
        data_to,
    );
    Ok(Json(response))
}
