use crate::types::{Timestamp, TxHash, H256};

mod memory;

pub trait DB {
    type Error;

    fn insert_tx(&mut self, tx: Tx) -> Result<(), Self::Error>;

    fn insert_block(&mut self, block: Block) -> Result<(), Self::Error>;

    fn insert_miss(&mut self, miss: Miss) -> Result<(), Self::Error>;

    fn insert_validator(&mut self, validator: Validator) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub struct Tx {
    hash: TxHash,
}

#[derive(Debug)]
pub struct Block {
    hash: H256,
    proposer_index: u64,
}

#[derive(Debug)]
pub struct Miss {
    tx: TxHash,
    block: H256,
    delay: Timestamp,
}

#[derive(Debug)]
pub struct Validator {
    index: u64,
}
