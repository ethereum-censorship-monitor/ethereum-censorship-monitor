use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use ethers::{
    providers::{Http, Middleware, Provider},
    types::Transaction,
};

use crate::{
    analyze::{
        check_base_fee_too_low, check_nonce_mismatch, check_not_enough_space, check_tip_too_low,
    },
    cli::Config,
    consensus_api::ConsensusProvider,
    nonce_cache::NonceCache,
    types::{BeaconBlock, ExecutionPayload, TxHash, GENESIS_TIME_SECONDS, H256},
    watch::NodeConfig,
};

pub async fn check_transaction(
    transaction_hash: TxHash,
    config: &Config,
    num_blocks: usize,
) -> Result<()> {
    let node_config = NodeConfig::from(config);
    let execution_provider = node_config.execution_http_provider();
    let consensus_provider = node_config.consensus_provider();
    let mut nonce_cache = NonceCache::new(
        node_config.execution_http_provider(),
        config.nonce_cache_size,
    );

    let transaction = execution_provider
        .get_transaction(transaction_hash)
        .await
        .wrap_err("failed to fetch transaction")?
        .ok_or(eyre!("transaction not found"))?;
    let block_number = transaction
        .block_number
        .ok_or(eyre!("missing block number"))?
        .as_u64();

    let block_numbers = (block_number - num_blocks as u64 - 1..block_number - 1).rev();
    for (i, n) in block_numbers.enumerate() {
        println!("Inclusion check in block {n}:");
        check_transaction_in_block(
            &transaction,
            n,
            &execution_provider,
            &consensus_provider,
            &mut nonce_cache,
        )
        .await?;
        if i < num_blocks {
            println!();
        }
    }
    Ok(())
}

pub async fn check_transaction_in_block(
    transaction: &Transaction,
    block_number: u64,
    execution_provider: &Provider<Http>,
    consensus_provider: &ConsensusProvider,
    nonce_cache: &mut NonceCache,
) -> Result<()> {
    let block = execution_provider
        .get_block(block_number)
        .await?
        .ok_or(eyre!("block not found"))?;
    let slot = (block.timestamp.as_u64() - GENESIS_TIME_SECONDS as u64) / 12;
    let beacon_block_without_root = consensus_provider.fetch_beacon_block_by_slot(slot).await?;
    let beacon_block = BeaconBlock::new(beacon_block_without_root, H256::zero());
    let exec = &beacon_block.body.execution_payload;

    let replaced = check_replaced(transaction, exec);
    let not_enough_space = check_not_enough_space(transaction, exec);
    let base_fee_too_low = check_base_fee_too_low(transaction, exec)?;
    let tip_too_low = check_tip_too_low(transaction, exec)?;
    let nonce_mismatch = check_nonce_mismatch(transaction, &beacon_block, nonce_cache).await?;

    println!("  replaced by others: {replaced}");
    println!("    not enough space: {not_enough_space}");
    println!("    base fee too low: {base_fee_too_low}");
    println!("         tip too low: {tip_too_low}");
    println!("      nonce mismatch: {nonce_mismatch}");

    Ok(())
}

fn check_replaced(transaction: &Transaction, exec: &ExecutionPayload<Transaction>) -> bool {
    for tx in &exec.transactions {
        if tx.from == transaction.from {
            return true;
        }
    }
    false
}
