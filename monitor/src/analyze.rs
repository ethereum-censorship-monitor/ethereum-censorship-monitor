use std::cmp::min;
use std::collections::{HashMap, HashSet};

use crate::db;
use crate::nonce_cache::{NonceCache, NonceCacheError};
use crate::pool::{Pool, TransactionWithVisibility};
use crate::types::{BeaconBlock, ExecutionPayload, Transaction, TxHash, U256, U64};
use std::time::{Duration, Instant};

/// Return the fields a transaction misses which prevents us from analyzing it, if any.
fn get_missing_transaction_fields(transaction: &Transaction) -> Option<Vec<String>> {
    let mut missing_fields = Vec::new();

    if transaction.transaction_type.is_none() {
        missing_fields.push(String::from("type"));
    } else {
        let transaction_type = transaction.transaction_type.unwrap();
        if transaction_type == U64::from(0) || transaction_type == U64::from(1) {
            if transaction.gas_price.is_none() {
                missing_fields.push(String::from("gasPrice"))
            }
        } else if transaction_type == U64::from(2) {
            if transaction.max_fee_per_gas.is_none() {
                missing_fields.push(String::from("maxFeePerGas"))
            }
            if transaction.max_priority_fee_per_gas.is_none() {
                missing_fields.push(String::from("maxPriorityFeePerGas"))
            }
        }
    }

    if missing_fields.len() > 0 {
        Some(missing_fields)
    } else {
        None
    }
}

/// Check that the transaction type is supported (i.e. 0, 1, or 2)
fn check_supported_type(transaction: &Transaction) -> bool {
    transaction.transaction_type.unwrap() <= U64::from(2)
}

/// Possible justified reasons why a transaction is not in a block.
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum NonInclusionReason {
    NotEnoughSpace,
    BaseFeeTooLow,
    TipTooLow,
    NonceMismatch,
}

/// Perform all inclusion checks except for nonce mismatch.
fn check_inclusion_without_nonce(
    transaction: &Transaction,
    exec: &ExecutionPayload<Transaction>,
) -> Option<NonInclusionReason> {
    if check_not_enough_space(transaction, exec) {
        Some(NonInclusionReason::NotEnoughSpace)
    } else if check_base_fee_too_low(transaction, exec) {
        Some(NonInclusionReason::BaseFeeTooLow)
    } else if check_tip_too_low(transaction, exec) {
        Some(NonInclusionReason::TipTooLow)
    } else {
        None
    }
}

/// Calculate the tip amount a transaction would pay in a block with given base fee. Panics if
/// required transaction fields are missing.
fn get_tip(transaction: &Transaction, base_fee: U256) -> U256 {
    let t = transaction.transaction_type.unwrap();
    if t == U64::from(0) || t == U64::from(1) {
        transaction.gas_price.unwrap() - base_fee
    } else if t == U64::from(2) {
        min(
            transaction.max_fee_per_gas.unwrap() - base_fee,
            transaction.max_priority_fee_per_gas.unwrap(),
        )
    } else {
        panic!("unsupported transaction type {}", t)
    }
}

/// Check if there is not enough space left in the block to include the transaction.
fn check_not_enough_space(transaction: &Transaction, exec: &ExecutionPayload<Transaction>) -> bool {
    let unused_gas = exec.gas_limit - exec.gas_used;
    transaction.gas > U256::from(unused_gas.as_u64())
}

/// Check if the transaction doesn't pay a high enough base fee. Panics if required transaction
/// fields are missing.
fn check_base_fee_too_low(transaction: &Transaction, exec: &ExecutionPayload<Transaction>) -> bool {
    let t = transaction.transaction_type.unwrap();
    let max_base_fee = if t == U64::from(0) || t == U64::from(1) {
        transaction.gas_price.unwrap()
    } else if t == U64::from(2) {
        transaction.max_fee_per_gas.unwrap()
    } else {
        U256::zero()
    };
    max_base_fee < exec.base_fee_per_gas
}

/// Check if the transaction doesn't pay a high enough tip. Panics if required transaction
/// fields are missing.
fn check_tip_too_low(transaction: &Transaction, exec: &ExecutionPayload<Transaction>) -> bool {
    let min_tip = get_min_tip(&exec.transactions, exec.base_fee_per_gas);
    get_tip(transaction, exec.base_fee_per_gas) < min_tip
}

/// Check if there is a mismatch between transaction and account nonce. Panics if required
/// transaction fields are missing.
async fn check_nonce_mismatch(
    transaction: &Transaction,
    beacon_block: &BeaconBlock<Transaction>,
    nonce_cache: &mut NonceCache,
) -> Result<bool, NonceCacheError> {
    let nonce = nonce_cache.get(&transaction.from, beacon_block).await?;
    Ok(nonce != transaction.nonce.as_u64())
}

/// Get the minimum tip of the given transactions. Transactions with missing required fields are
/// ignored. If there's no transactions to consider, returns the maximum of U256.
fn get_min_tip(transactions: &Vec<Transaction>, base_fee: U256) -> U256 {
    transactions
        .iter()
        .filter(|tx| get_missing_transaction_fields(tx).is_none())
        .map(|tx| get_tip(tx, base_fee))
        .min()
        .unwrap_or(U256::MAX)
}

#[derive(Debug)]
pub struct Analysis {
    pub beacon_block: BeaconBlock<Transaction>,
    pub missing_transactions: HashMap<TxHash, TransactionWithVisibility>,
    pub included_transactions: HashMap<TxHash, TransactionWithVisibility>,
    pub num_txs_in_block: usize,
    pub num_txs_in_pool: usize,
    pub num_only_tx_hash: usize,
    pub non_inclusion_reasons: HashMap<NonInclusionReason, usize>,
    pub duration: Duration,
}

impl Analysis {
    pub fn summary(&self) -> String {
        format!(
            "Analysis for block {}: {} txs from pool included, {} missed, {} in pool, {} in block, {} only tx hash, {} nonce mismatch, {} not enough space, {} base fee too low, {} tip too low, took {}s",
            self.beacon_block,
            self.included_transactions.len(),
            self.missing_transactions.len(),
            self.num_txs_in_pool,
            self.num_txs_in_block,
            self.non_inclusion_reasons.get(&NonInclusionReason::NonceMismatch).unwrap_or(&0),
            self.non_inclusion_reasons.get(&NonInclusionReason::NotEnoughSpace).unwrap_or(&0),
            self.non_inclusion_reasons.get(&NonInclusionReason::BaseFeeTooLow).unwrap_or(&0),
            self.non_inclusion_reasons.get(&NonInclusionReason::TipTooLow).unwrap_or(&0),
            self.num_only_tx_hash,
            self.duration.as_secs(),
        )
    }
}

pub async fn analyze(
    beacon_block: &BeaconBlock<Transaction>,
    pool: &Pool,
    nonce_cache: &mut NonceCache,
) -> Result<Analysis, NonceCacheError> {
    let start_time = Instant::now();

    let exec = &beacon_block.body.execution_payload;
    let txs_in_block: HashSet<&TxHash> =
        HashSet::from_iter(exec.transactions.iter().map(|tx| &tx.hash));
    let proposal_time = beacon_block.proposal_time();
    let pool_at_t = pool.content_at(proposal_time);

    let num_txs_in_block = exec.transactions.len();
    let num_txs_in_pool = pool_at_t.len();

    let mut missing_txs = HashMap::new();
    let mut included_txs = HashMap::new();
    let mut non_inclusion_reasons = HashMap::new();
    let mut num_only_tx_hash = 0;

    for (hash, tx_with_vis) in pool_at_t {
        if txs_in_block.contains(&hash) {
            included_txs.insert(hash, tx_with_vis);
        } else {
            if let Some(ref tx) = tx_with_vis.transaction {
                if let Some(missing_fields) = get_missing_transaction_fields(tx) {
                    log::warn!(
                        "skipping transaction with missing required fields {}: {:?}",
                        missing_fields.join(", "),
                        tx
                    );
                } else if !check_supported_type(tx) {
                    log::warn!(
                        "skipping transaction with unsupported type {}",
                        tx.transaction_type.unwrap()
                    );
                } else if let Some(reason) = check_inclusion_without_nonce(tx, exec) {
                    *non_inclusion_reasons.entry(reason).or_insert(0) += 1;
                } else if check_nonce_mismatch(tx, beacon_block, nonce_cache).await? {
                    *non_inclusion_reasons
                        .entry(NonInclusionReason::NonceMismatch)
                        .or_insert(0) += 1;
                } else {
                    missing_txs.insert(hash, tx_with_vis);
                }
            } else {
                num_only_tx_hash += 1;
            }
        }
    }

    let duration = start_time.elapsed();

    Ok(Analysis {
        beacon_block: beacon_block.clone(),
        missing_transactions: missing_txs,
        included_transactions: included_txs,
        num_txs_in_block,
        num_txs_in_pool,
        num_only_tx_hash,
        non_inclusion_reasons,
        duration,
    })
}

pub fn insert_analysis_into_db<T: db::DB>(analysis: &Analysis, db: &mut T) -> Result<(), T::Error> {
    // let block_hash = analysis.block.hash.unwrap();
    // for (hash, tx) in &analysis.missing_transactions {
    //     db.insert_tx(db::Tx { hash: *hash })?;
    //     db.insert_block(db::Block {
    //         hash: block_hash,
    //         proposer_index: 0,
    //     })?;
    //     db.insert_miss(db::Miss {
    //         tx: *hash,
    //         block: block_hash,
    //         delay: tx.first_seen - analysis.block.timestamp.as_u64(),
    //     })?;
    // }
    Ok(())
}
