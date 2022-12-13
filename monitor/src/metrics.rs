use lazy_static::lazy_static;
use prometheus::{
    opts, register_gauge, register_int_counter, register_int_counter_vec, register_int_gauge,
    Encoder, Gauge, IntCounter, IntCounterVec, IntGauge,
};
use warp::Filter;

use crate::cli::Config;

lazy_static! {
    pub static ref TXS_IN_POOL: IntGauge =
        register_int_gauge!("txs_in_pool", "Transactions in pool").expect("can create metric");
    pub static ref HEAD_HISTORY_LENGTH: IntGauge =
        register_int_gauge!(opts!("head_history_length", "Head history length"))
            .expect("can create metric");
    pub static ref TXS_FROM_PROVIDERS: IntCounterVec = register_int_counter_vec!(
        "txs_from_providers",
        "Transactions from providers",
        &["index"]
    )
    .expect("can create metric");
    pub static ref BLOCKS: IntCounter =
        register_int_counter!("blocks", "Blocks").expect("can create metric");
    pub static ref NONCE_CACHE_SIZE: IntGauge =
        register_int_gauge!(opts!("nonce_cache_size", "Nonce cache size"))
            .expect("can create metric");
    pub static ref EVENT_CHANNEL_CAPACITY: Gauge =
        register_gauge!("event_channel_capacity", "Event channel capacity")
            .expect("can create metric");
    pub static ref FETCH_BLOCK_DURATION: Gauge =
        register_gauge!("fetch_block_duration", "Fetch block duration (s)")
            .expect("can create metric");
    pub static ref FETCH_POOL_DURATION: Gauge =
        register_gauge!("fetch_pool_duration", "Fetch pool duration (s)")
            .expect("can create metric");
    pub static ref ANALYSIS_DURATION: Gauge =
        register_gauge!("analysis_duration", "Analysis duration (s)").expect("can create metric");
    pub static ref TRANSACTIONS_IN_BLOCKS: IntCounter =
        register_int_counter!("transactions_in_blocks", "Transactions in blocks")
            .expect("can create metric");
    pub static ref ANALYZED_TRANSACTIONS: IntCounter = register_int_counter!(
        "analyzed_transactions",
        "Transactions from pool that were analyzed"
    )
    .expect("can create metric");
    pub static ref INCLUDED_TRANSACTIONS: IntCounter = register_int_counter!(
        "included_transactions",
        "Analyzed transactions that were included in block"
    )
    .expect("can create metric");
    pub static ref QUORUM_NOT_REACHED_TRANSACTIONS: IntCounter = register_int_counter!(
        "quorum_not_reached_transactions",
        "Analyzed transactions that didn't reach quorum"
    )
    .expect("can create metric");
    pub static ref ONLY_HASH_TRANSACTIONS: IntCounter = register_int_counter!(
        "only_hash_transactions",
        "Analyzed transactions of which only their hash is known"
    )
    .expect("can create metric");
    pub static ref REPLACED_TRANSACTIONS: IntCounter = register_int_counter!(
        "replaced_transactions",
        "Analyzed transactions that got replaced by other transactions of the same sender"
    )
    .expect("can create metric");
    pub static ref NOT_ENOUGH_SPACE_TRANSACTIONS: IntCounter = register_int_counter!(
        "not_enough_space_transactions",
        "Analyzed transactions for which there wasn't enough space in the block"
    )
    .expect("can create metric");
    pub static ref BASE_FEE_TOO_LOW_TRANSACTIONS: IntCounter = register_int_counter!(
        "base_fee_too_low_transactions",
        "Analyzed transactions whose base fee was too low"
    )
    .expect("can create metric");
    pub static ref TIP_TOO_LOW_TRANSACTIONS: IntCounter = register_int_counter!(
        "tip_too_low_transactions",
        "Analyzed transactions whose tip was too small"
    )
    .expect("can create metric");
    pub static ref NONCE_MISMATCH_TRANSACTIONS: IntCounter = register_int_counter!(
        "nonce_mismatch_transactions",
        "Analyzed transactions whose nonce was incorrect"
    )
    .expect("can create metric");
    pub static ref MISSING_TRANSACTIONS: IntCounter = register_int_counter!(
        "missing_transactions",
        "Analyzed transactions that should have been included but weren't"
    )
    .expect("can create metric");
}

pub async fn serve(config: &Config) {
    let route = warp::path!("metrics").and_then(handler);
    warp::serve(route).run(config.metrics_endpoint).await;
}

async fn handler() -> Result<String, warp::Rejection> {
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();
    encoder
        .encode(&prometheus::gather(), &mut buffer)
        .expect("can encode metrics");
    let response = String::from_utf8(buffer.clone()).expect("can convert metrics to string");
    buffer.clear();
    Ok(response)
}
