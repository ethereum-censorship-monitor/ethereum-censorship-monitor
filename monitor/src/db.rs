use crate::types::{Timestamp, TxHash, H256};

pub mod memory;

pub trait DB {
    type Error;

    fn insert_tx(&mut self, tx: Tx) -> Result<(), Self::Error>;

    fn insert_block(&mut self, block: Block) -> Result<(), Self::Error>;

    fn insert_miss(&mut self, miss: Miss) -> Result<(), Self::Error>;

    fn insert_validator(&mut self, validator: Validator) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub struct Tx {
    pub hash: TxHash,
}

#[derive(Debug)]
pub struct Block {
    pub hash: H256,
    pub proposer_index: u64,
}

#[derive(Debug)]
pub struct Miss {
    pub tx: TxHash,
    pub block: H256,
    pub delay: Timestamp,
}

#[derive(Debug)]
pub struct Validator {
    pub index: u64,
}
