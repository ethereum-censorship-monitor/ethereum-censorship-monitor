use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::{
    metrics,
    nonce_cache::{NonceCache, NonceCacheError},
    pool::{ObservedTransaction, Pool},
    types::{Address, BeaconBlock, ExecutionPayload, Transaction, TxHash, U256},
};

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
pub enum TransactionError {
    #[error("transaction is missing required field {name}")]
    MissingRequiredField { name: String },
    #[error("transaction has type {transaction_type} which is not supported")]
    UnsupportedType { transaction_type: u64 },
    #[error("transaction fee is lower than base fee")]
    FeeTooLow {
        max_fee: U256,
        base_fee: U256,
        transaction_type: u64,
    },
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

/// Calculate the tip amount a transaction would pay in a block with given base
/// fee.
pub fn get_tip(transaction: &Transaction, base_fee: U256) -> Result<U256, TransactionError> {
    let t = get_transaction_type(transaction)?;
    if t == 0 || t == 1 {
        let gas_price = transaction
            .gas_price
            .ok_or(TransactionError::MissingRequiredField {
                name: String::from("gasPrice"),
            })?;
        if gas_price < base_fee {
            Err(TransactionError::FeeTooLow {
                max_fee: gas_price,
                base_fee,
                transaction_type: t,
            })
        } else {
            Ok(gas_price - base_fee)
        }
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
        if max_fee_per_gas < base_fee {
            Err(TransactionError::FeeTooLow {
                max_fee: max_fee_per_gas,
                base_fee,
                transaction_type: t,
            })
        } else {
            Ok(min(max_fee_per_gas - base_fee, max_priority_fee_per_gas))
        }
    } else {
        Err(TransactionError::UnsupportedType {
            transaction_type: t,
        })
    }
}

/// Check if there is not enough space left in the block to include the
/// transaction.
pub fn check_not_enough_space(
    transaction: &Transaction,
    exec: &ExecutionPayload<Transaction>,
) -> bool {
    let unused_gas = exec.gas_limit - exec.gas_used;
    transaction.gas > U256::from(unused_gas.as_u64())
}

/// Check if the transaction doesn't pay a high enough base fee.
pub fn check_base_fee_too_low(
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
pub fn check_tip_too_low(
    transaction: &Transaction,
    exec: &ExecutionPayload<Transaction>,
) -> Result<bool, TransactionError> {
    let median_tip = get_median_tip(&exec.transactions, exec.base_fee_per_gas);
    match get_tip(transaction, exec.base_fee_per_gas) {
        Ok(tip) => Ok(tip < median_tip),
        Err(TransactionError::FeeTooLow {
            max_fee: _,
            base_fee: _,
            transaction_type: _,
        }) => Ok(true),
        Err(e) => Err(e),
    }
}

/// Check if there is a mismatch between transaction and account nonce.
pub async fn check_nonce_mismatch(
    transaction: &Transaction,
    beacon_block: &BeaconBlock<Transaction>,
    nonce_cache: &mut NonceCache,
) -> Result<bool, NonceCacheError> {
    let nonce = nonce_cache.get(&transaction.from, beacon_block).await?;
    Ok(nonce != transaction.nonce.as_u64())
}

/// Get the minimum tip of the given transactions. Transactions with missing
/// required fields are ignored. If there's no transactions to consider, returns
/// the maximum of U256.
pub fn get_min_nonzero_tip(transactions: &[Transaction], base_fee: U256) -> U256 {
    transactions
        .iter()
        .filter_map(|tx| get_tip(tx, base_fee).ok())
        .filter(|tip| !tip.is_zero())
        .min()
        .unwrap_or(U256::MAX)
}

/// Get the median tip amount of the given transactions. Transactions with
/// missing required fields are ignored. If there's no transactions to consider,
/// returns the maximum of U256.
pub fn get_median_tip(transactions: &[Transaction], base_fee: U256) -> U256 {
    let mut tips: Vec<U256> = transactions
        .iter()
        .filter_map(|tx| get_tip(tx, base_fee).ok())
        .collect();
    let n = tips.len();
    if n == 0 {
        U256::MAX
    } else {
        tips.sort();
        if n % 2 == 0 {
            (tips[n / 2 - 1] + tips[n / 2]) / 2
        } else {
            tips[(n - 1) / 2]
        }
    }
}

#[derive(Debug)]
pub struct Analysis {
    pub beacon_block: BeaconBlock<Transaction>,
    pub quorum: usize,
    pub missing_transactions: HashMap<TxHash, MissedTransaction>,
    pub included_transactions: HashMap<TxHash, ObservedTransaction>,
    pub num_txs_in_block: usize,
    pub num_txs_in_pool: usize,
    pub num_quorum_not_reached: usize,
    pub num_still_propagating: usize,
    pub num_only_tx_hash: usize,
    pub num_replaced_txs: usize,
    pub non_inclusion_reasons: HashMap<NonInclusionReason, usize>,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct MissedTransaction {
    pub hash: TxHash,
    pub transaction: Transaction,
    pub first_seen: DateTime<Utc>,
    pub quorum_reached: DateTime<Utc>,
    pub tip: i64,
}

impl Analysis {
    pub fn summary(&self) -> String {
        format!(
            "Analysis for block {beacon_block}: {included} txs from pool included, {missing} \
             missed, {in_pool} in pool, {in_block} in block, {quorum_not_reached} quorum not \
             reached, {still_propagating} still propagating, {only_hash} only hash known, \
             {replaced} replaced, {nonce_mismatch} nonce mismatch, {not_enough_space} not enough \
             space, {base_fee_too_low} base fee too low, {tip_too_low} tip too low, took \
             {duration:.1}s",
            beacon_block = self.beacon_block,
            included = self.included_transactions.len(),
            missing = self.missing_transactions.len(),
            in_pool = self.num_txs_in_pool,
            in_block = self.num_txs_in_block,
            quorum_not_reached = self.num_quorum_not_reached,
            still_propagating = self.num_still_propagating,
            only_hash = self.num_only_tx_hash,
            replaced = self.num_replaced_txs,
            nonce_mismatch = self
                .non_inclusion_reasons
                .get(&NonInclusionReason::NonceMismatch)
                .unwrap_or(&0),
            not_enough_space = self
                .non_inclusion_reasons
                .get(&NonInclusionReason::NotEnoughSpace)
                .unwrap_or(&0),
            base_fee_too_low = self
                .non_inclusion_reasons
                .get(&NonInclusionReason::BaseFeeTooLow)
                .unwrap_or(&0),
            tip_too_low = self
                .non_inclusion_reasons
                .get(&NonInclusionReason::TipTooLow)
                .unwrap_or(&0),
            duration = self.duration.as_secs_f64(),
        )
    }
}

pub async fn analyze(
    beacon_block: &BeaconBlock<Transaction>,
    pool: &Pool,
    nonce_cache: &mut NonceCache,
    quorum: usize,
    propagation_time: chrono::Duration,
) -> Result<Analysis, NonceCacheError> {
    let start_time = Instant::now();

    let exec = &beacon_block.body.execution_payload;
    let mut txs_in_block: HashSet<&TxHash> = HashSet::new();
    let mut senders_in_block: HashSet<Address> = HashSet::new();
    for tx in &exec.transactions {
        txs_in_block.insert(&tx.hash);
        let sender = tx.recover_from();
        match sender {
            Err(e) => {
                log::warn!("failed to recover sender address of tx {}: {e}", tx.hash);
            }
            Ok(sender) => {
                senders_in_block.insert(sender);
            }
        }
    }
    let proposal_time = beacon_block.proposal_time();
    let pool_at_t = pool.content_at(proposal_time);

    let num_txs_in_block = exec.transactions.len();
    let num_txs_in_pool = pool_at_t.len();

    let mut included_txs = HashMap::new();
    let mut num_only_tx_hash = 0;
    let mut num_quorum_not_reached = 0;
    let mut num_still_propagating = 0;
    let mut num_replaced_txs = 0;
    let mut missing_txs = HashMap::new();
    let mut non_inclusion_reasons = HashMap::new();

    for (hash, obs_tx) in pool_at_t {
        if txs_in_block.contains(&hash) {
            included_txs.insert(hash, obs_tx);
            continue;
        }
        if obs_tx.num_nodes_seen(proposal_time) < quorum {
            num_quorum_not_reached += 1;
            continue;
        }
        let first_seen = obs_tx
            .quorum_reached_timestamp(1)
            .expect("quorum has been reached");
        let quorum_reached = obs_tx
            .quorum_reached_timestamp(quorum)
            .expect("quorum has been reached");
        if beacon_block.proposal_time() - quorum_reached <= propagation_time {
            num_still_propagating += 1;
            continue;
        }
        if obs_tx.transaction.is_none() {
            num_only_tx_hash += 1;
            continue;
        }
        let tx = obs_tx.transaction.as_ref().unwrap();
        if let Ok(from) = tx.recover_from() {
            if senders_in_block.contains(&from) {
                num_replaced_txs += 1;
                continue;
            }
        } else {
            log::warn!("failed to recover sender address of tx {}", tx.hash);
        }

        match check_inclusion(tx, beacon_block, nonce_cache).await {
            Ok(Some(reason)) => *non_inclusion_reasons.entry(reason).or_insert(0) += 1,
            Ok(None) => {
                if obs_tx.transaction.is_none() {
                    log::error!("transaction without body failed inclusion checks");
                    continue;
                }
                let tx = obs_tx.transaction.unwrap();
                let tip = get_tip(&tx, beacon_block.body.execution_payload.base_fee_per_gas);
                if let Err(e) = tip {
                    log::error!(
                        "transaction whose tip we cannot determine failed inclusion checks ({})",
                        e
                    );
                    continue;
                }
                let tip = tip.unwrap();
                if tip > U256::from(i64::MAX) {
                    log::warn!("ignoring tx with huge tip");
                    continue;
                }
                let tip = tip.as_u64() as i64;
                let missed_tx = MissedTransaction {
                    hash: obs_tx.hash,
                    transaction: tx,
                    first_seen,
                    quorum_reached,
                    tip,
                };
                missing_txs.insert(hash, missed_tx);
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
    metrics::ANALYSIS_DURATION.set(duration.as_millis() as f64 / 1000.);
    metrics::TRANSACTIONS_IN_BLOCKS.inc_by(txs_in_block.len() as u64);
    metrics::ANALYZED_TRANSACTIONS.inc_by(num_txs_in_pool as u64);
    metrics::INCLUDED_TRANSACTIONS.inc_by(included_txs.len() as u64);
    metrics::QUORUM_NOT_REACHED_TRANSACTIONS.inc_by(num_quorum_not_reached as u64);
    metrics::STILL_PROPAGATING_TRANSACTIONS.inc_by(num_still_propagating as u64);
    metrics::ONLY_HASH_TRANSACTIONS.inc_by(num_only_tx_hash as u64);
    metrics::REPLACED_TRANSACTIONS.inc_by(num_replaced_txs as u64);
    metrics::NOT_ENOUGH_SPACE_TRANSACTIONS.inc_by(
        *non_inclusion_reasons
            .get(&NonInclusionReason::NotEnoughSpace)
            .unwrap_or(&0) as u64,
    );
    metrics::BASE_FEE_TOO_LOW_TRANSACTIONS.inc_by(
        *non_inclusion_reasons
            .get(&NonInclusionReason::BaseFeeTooLow)
            .unwrap_or(&0) as u64,
    );
    metrics::TIP_TOO_LOW_TRANSACTIONS.inc_by(
        *non_inclusion_reasons
            .get(&NonInclusionReason::TipTooLow)
            .unwrap_or(&0) as u64,
    );
    metrics::NONCE_MISMATCH_TRANSACTIONS.inc_by(
        *non_inclusion_reasons
            .get(&NonInclusionReason::NonceMismatch)
            .unwrap_or(&0) as u64,
    );
    metrics::MISSING_TRANSACTIONS.inc_by(missing_txs.len() as u64);

    Ok(Analysis {
        beacon_block: beacon_block.clone(),
        quorum,
        missing_transactions: missing_txs,
        included_transactions: included_txs,
        num_txs_in_block,
        num_quorum_not_reached,
        num_still_propagating,
        num_txs_in_pool,
        num_only_tx_hash,
        num_replaced_txs,
        non_inclusion_reasons,
        duration,
    })
}
