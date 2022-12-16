use color_eyre::Result;
use ethers::providers::Middleware;

use crate::{
    analyze::{get_median_tip, get_min_nonzero_tip, get_tip},
    cli::Config,
    types::{TxHash, U256},
    watch::NodeConfig,
};

pub async fn check_gas(config: Config, tx_hash: TxHash, slot: u64) -> Result<()> {
    let node_config = NodeConfig::from(&config);
    let ep = node_config.execution_http_provider();
    let cp = node_config.consensus_provider();

    let tx = ep.get_transaction(tx_hash).await?.unwrap();
    let b = cp.fetch_beacon_block_by_slot(slot).await?;
    let exec = b.body.execution_payload;

    let block_base_fee = exec.base_fee_per_gas;
    let min_tip = get_min_nonzero_tip(&exec.transactions, block_base_fee);
    let median_tip = get_median_tip(&exec.transactions, block_base_fee);

    let tx_type = tx.transaction_type.unwrap().as_u64();
    let tx_base_fee = match tx_type {
        0 => tx.gas_price,
        2 => tx.max_fee_per_gas,
        _ => panic!("unknown tx type"),
    }
    .unwrap();
    let tx_tip = get_tip(&tx, block_base_fee).unwrap();

    let block_gas_limit = exec.gas_limit;
    let block_gas_used = exec.gas_used;
    let tx_gas_limit = tx.gas;

    fn gwei(i: U256) -> String {
        format!("{:.2}gwei", i.as_u64() as f64 / 1_000_000_000.)
    }

    println!("         tx type: {}", tx_type);
    println!("  block base fee: {}", gwei(block_base_fee));
    println!("     tx base fee: {}", gwei(tx_base_fee));
    println!("   block min tip: {}", gwei(min_tip));
    println!("block median tip: {}", gwei(median_tip));
    println!("          tx tip: {}", gwei(tx_tip));
    println!(" block gas limit: {}", block_gas_limit);
    println!("  block gas used: {}", block_gas_used);
    println!("block unused gas: {}", block_gas_limit - block_gas_used);
    println!("    tx gas limit: {}", tx_gas_limit);
    Ok(())
}
