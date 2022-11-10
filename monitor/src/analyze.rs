use std::cmp::min;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::nonce_cache::{NonceCache, NonceCacheError};
use crate::pool::{Pool, TransactionWithVisibility};
use crate::types::{BeaconBlock, ExecutionPayload, Transaction, TxHash, U256};
use std::time::{Duration, Instant};

/// Possible justified reasons why a transaction is not in a block.
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum NonInclusionReason {
    NotEnoughSpace,
    BaseFeeTooLow,
    TipTooLow,
    NonceMismatch,
}

#[derive(Debug, Error)]
enum InclusionCheckError {
    #[error("cannot check inclusion as transaction is missing required field")]
    TransactionError(#[from] TransactionError),
    #[error("cannot check inclusion due to nonce cache error")]
    NonceCacheError(#[from] NonceCacheError),
}

#[derive(Debug, Error)]
enum TransactionError {
    #[error("transaction is missing required field {name}")]
    MissingRequiredField { name: String },
    #[error("transaction has type {transaction_type} which is not supported")]
    UnsupportedType { transaction_type: u64 },
}

/// Perform all inclusion checks.
async fn check_inclusion(
    transaction: &Transaction,
    beacon_block: &BeaconBlock<Transaction>,
    nonce_cache: &mut NonceCache,
) -> Result<Option<NonInclusionReason>, InclusionCheckError> {
    let exec = &beacon_block.body.execution_payload;
    if check_not_enough_space(transaction, exec) {
        Ok(Some(NonInclusionReason::NotEnoughSpace))
    } else if check_base_fee_too_low(transaction, exec)? {
        Ok(Some(NonInclusionReason::BaseFeeTooLow))
    } else if check_tip_too_low(transaction, exec)? {
        Ok(Some(NonInclusionReason::TipTooLow))
    } else if check_nonce_mismatch(transaction, beacon_block, nonce_cache).await? {
        Ok(Some(NonInclusionReason::NonceMismatch))
    } else {
        Ok(None)
    }
}

/// Get the type of the transaction or an error if it is not specified.
fn get_transaction_type(transaction: &Transaction) -> Result<u64, TransactionError> {
    match transaction.transaction_type {
        Some(t) => Ok(t.as_u64()),
        None => Err(TransactionError::MissingRequiredField {
            name: String::from("type"),
        }),
    }
}

/// Calculate the tip amount a transaction would pay in a block with given base fee.
fn get_tip(transaction: &Transaction, base_fee: U256) -> Result<U256, TransactionError> {
    let t = get_transaction_type(transaction)?;
    if t == 0 || t == 1 {
        let gas_price = transaction
            .gas_price
            .ok_or(TransactionError::MissingRequiredField {
                name: String::from("gasPrice"),
            })?;
        Ok(gas_price - base_fee)
    } else if t == 2 {
        let max_fee_per_gas =
            transaction
                .max_fee_per_gas
                .ok_or(TransactionError::MissingRequiredField {
                    name: String::from("maxFeePerGas"),
                })?;
        let max_priority_fee_per_gas =
            transaction
                .max_priority_fee_per_gas
                .ok_or(TransactionError::MissingRequiredField {
                    name: String::from("maxPriorityFeePerGas"),
                })?;
        Ok(min(max_fee_per_gas - base_fee, max_priority_fee_per_gas))
    } else {
        Err(TransactionError::UnsupportedType {
            transaction_type: t,
        })
    }
}

/// Check if there is not enough space left in the block to include the transaction.
fn check_not_enough_space(transaction: &Transaction, exec: &ExecutionPayload<Transaction>) -> bool {
    let unused_gas = exec.gas_limit - exec.gas_used;
    transaction.gas > U256::from(unused_gas.as_u64())
}

/// Check if the transaction doesn't pay a high enough base fee.
fn check_base_fee_too_low(
    transaction: &Transaction,
    exec: &ExecutionPayload<Transaction>,
) -> Result<bool, TransactionError> {
    let t = get_transaction_type(transaction)?;
    let max_base_fee = if t == 0 || t == 1 {
        transaction
            .gas_price
            .ok_or(TransactionError::MissingRequiredField {
                name: String::from("gasPrice"),
            })?
    } else if t == 2 {
        transaction
            .max_fee_per_gas
            .ok_or(TransactionError::MissingRequiredField {
                name: String::from("maxFeePerGas"),
            })?
    } else {
        return Err(TransactionError::UnsupportedType {
            transaction_type: t,
        });
    };
    Ok(max_base_fee < exec.base_fee_per_gas)
}

/// Check if the transaction doesn't pay a high enough tip.
fn check_tip_too_low(
    transaction: &Transaction,
    exec: &ExecutionPayload<Transaction>,
) -> Result<bool, TransactionError> {
    let min_tip = get_min_tip(&exec.transactions, exec.base_fee_per_gas);
    Ok(get_tip(transaction, exec.base_fee_per_gas)? < min_tip)
}

/// Check if there is a mismatch between transaction and account nonce.
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
        .filter_map(|tx| get_tip(tx, base_fee).ok())
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
            continue;
        }
        if tx_with_vis.transaction.is_none() {
            num_only_tx_hash += 1;
            continue;
        }
        let tx = tx_with_vis.transaction.as_ref().unwrap();

        match check_inclusion(&tx, beacon_block, nonce_cache).await {
            Ok(Some(reason)) => *non_inclusion_reasons.entry(reason).or_insert(0) += 1,
            Ok(None) => {
                missing_txs.insert(hash, tx_with_vis);
            }
            Err(InclusionCheckError::TransactionError(e)) => {
                log::warn!(
                    "failed to check inclusion criteria for tx {}: {} (tx: {:?})",
                    tx.hash,
                    e,
                    tx,
                )
            }
            Err(InclusionCheckError::NonceCacheError(e)) => {
                return Err(e);
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
