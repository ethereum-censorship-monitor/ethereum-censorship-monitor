use std::{fmt, path::Path};

use chrono::{DateTime, Duration, TimeZone, Utc};
use ethers::abi::ethereum_types::FromDecStrErr;
pub use ethers::types::{
    Address, Block, Bytes, Transaction, TxHash, TxpoolContent, H256, U256, U64,
};
use serde::Deserialize;
use url::Url;

pub const GENESIS_TIME_SECONDS: i64 = 1606824023;

pub type NodeKey = usize;

trait FromDecStr: Sized {
    fn from_dec_str(value: &str) -> Result<Self, FromDecStrErr>;
}

impl FromDecStr for U256 {
    fn from_dec_str(value: &str) -> Result<Self, FromDecStrErr> {
        U256::from_dec_str(value)
    }
}

impl FromDecStr for U64 {
    fn from_dec_str(value: &str) -> Result<Self, FromDecStrErr> {
        U64::from_dec_str(value)
    }
}

fn from_dec_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromDecStr,
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_dec_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct NewBeaconHeadEvent {
    #[serde(deserialize_with = "from_dec_str")]
    pub slot: U256,
    pub block: H256,
    pub state: H256,
    pub current_duty_dependent_root: H256,
    pub previous_duty_dependent_root: H256,
    pub epoch_transition: bool,
    pub execution_optimistic: bool,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct SignedMessage<T> {
    pub message: T,
    pub signature: Bytes,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct BeaconBlockWithoutRoot<T> {
    #[serde(deserialize_with = "from_dec_str")]
    pub slot: U64,
    #[serde(deserialize_with = "from_dec_str")]
    pub proposer_index: U64,
    pub parent_root: H256,
    pub state_root: H256,
    pub body: BeaconBlockBody<T>,
}

impl<T> BeaconBlockWithoutRoot<T> {
    pub fn with_transactions<S>(b: Self, txs: Vec<S>) -> BeaconBlockWithoutRoot<S> {
        BeaconBlockWithoutRoot {
            slot: b.slot,
            proposer_index: b.proposer_index,
            parent_root: b.parent_root,
            state_root: b.state_root,
            body: BeaconBlockBody::with_transactions(b.body, txs),
        }
    }
}

impl<T> fmt::Display for BeaconBlockWithoutRoot<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "#{}", self.slot)
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct BeaconBlock<T> {
    pub root: H256,
    #[serde(deserialize_with = "from_dec_str")]
    pub slot: U64,
    #[serde(deserialize_with = "from_dec_str")]
    pub proposer_index: U64,
    pub parent_root: H256,
    pub state_root: H256,
    pub body: BeaconBlockBody<T>,
}

impl<T> BeaconBlock<T> {
    pub fn new(b: BeaconBlockWithoutRoot<T>, root: H256) -> BeaconBlock<T> {
        BeaconBlock {
            root,
            slot: b.slot,
            proposer_index: b.proposer_index,
            parent_root: b.parent_root,
            state_root: b.state_root,
            body: b.body,
        }
    }

    pub fn proposal_time(&self) -> DateTime<Utc> {
        Utc.timestamp_opt(GENESIS_TIME_SECONDS, 0).unwrap()
            + Duration::seconds((self.slot.as_u64() * 12) as i64)
    }
}

impl<T> fmt::Display for BeaconBlock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "#{}:{}", self.slot, self.root)
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct BeaconBlockBody<T> {
    pub randao_reveal: Bytes,
    #[serde(skip)]
    pub eth1_data: (),
    pub graffiti: H256,
    #[serde(skip)]
    pub proposer_slashings: (),
    #[serde(skip)]
    pub attester_slashings: (),
    #[serde(skip)]
    pub attestations: (),
    #[serde(skip)]
    pub deposits: (),
    #[serde(skip)]
    pub voluntary_exits: (),
    #[serde(skip)]
    pub sync_aggregate: (),
    pub execution_payload: ExecutionPayload<T>,
}

impl<T> BeaconBlockBody<T> {
    pub fn with_transactions<S>(b: Self, txs: Vec<S>) -> BeaconBlockBody<S> {
        BeaconBlockBody {
            randao_reveal: b.randao_reveal,
            eth1_data: b.eth1_data,
            graffiti: b.graffiti,
            proposer_slashings: b.proposer_slashings,
            attester_slashings: b.attester_slashings,
            attestations: b.attestations,
            deposits: b.deposits,
            voluntary_exits: b.voluntary_exits,
            sync_aggregate: b.sync_aggregate,
            execution_payload: ExecutionPayload::with_transactions(b.execution_payload, txs),
        }
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct ExecutionPayload<T> {
    pub parent_hash: H256,
    pub fee_recipient: Address,
    pub state_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: Bytes,
    pub prev_randao: H256,
    #[serde(deserialize_with = "from_dec_str")]
    pub block_number: U64,
    #[serde(deserialize_with = "from_dec_str")]
    pub gas_limit: U64,
    #[serde(deserialize_with = "from_dec_str")]
    pub gas_used: U64,
    #[serde(deserialize_with = "from_dec_str")]
    pub timestamp: U64,
    pub extra_data: Bytes,
    #[serde(deserialize_with = "from_dec_str")]
    pub base_fee_per_gas: U256,
    pub block_hash: H256,
    pub transactions: Vec<T>,
}

impl<T> ExecutionPayload<T> {
    pub fn with_transactions<S>(e: Self, txs: Vec<S>) -> ExecutionPayload<S> {
        ExecutionPayload {
            parent_hash: e.parent_hash,
            fee_recipient: e.fee_recipient,
            state_root: e.state_root,
            receipts_root: e.receipts_root,
            logs_bloom: e.logs_bloom,
            prev_randao: e.prev_randao,
            block_number: e.block_number,
            gas_limit: e.gas_limit,
            gas_used: e.gas_used,
            timestamp: e.timestamp,
            extra_data: e.extra_data,
            base_fee_per_gas: e.base_fee_per_gas,
            block_hash: e.block_hash,
            transactions: txs,
        }
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ConsensusSyncStatus {
    #[serde(deserialize_with = "from_dec_str")]
    pub head_slot: U256,
    #[serde(deserialize_with = "from_dec_str")]
    pub sync_distance: U256,
    pub is_syncing: bool,
    pub is_optimistic: bool,
}

/// Join a url with optional path with an sub-path. This function is needed
/// because Url::join strips an existing path on the URL if the sub-path starts
/// with a slash.
pub fn url_with_path(url: &Url, path: &str) -> Url {
    let relative_path = path.strip_prefix('/').unwrap_or(path);
    let full_path = Path::new(url.path()).join(relative_path);

    let mut url = url.clone();
    url.set_path(full_path.to_str().unwrap());
    url
}
