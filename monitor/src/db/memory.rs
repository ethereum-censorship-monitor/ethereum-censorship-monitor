use crate::types::{TxHash, H256};
use std::collections::HashMap;

use super::{Block, Miss, Tx, Validator, DB};

pub struct MemoryDB {
    txs: HashMap<TxHash, Tx>,
    blocks: HashMap<TxHash, Block>,
    misses: HashMap<(TxHash, H256), Miss>,
    validators: HashMap<u64, Validator>,
}

impl MemoryDB {
    pub fn new() -> Self {
        MemoryDB {
            txs: HashMap::new(),
            blocks: HashMap::new(),
            misses: HashMap::new(),
            validators: HashMap::new(),
        }
    }
}

impl DB for MemoryDB {
    type Error = ();

    fn insert_tx(&mut self, tx: Tx) -> Result<(), Self::Error> {
        self.txs.insert(tx.hash, tx);
        Ok(())
    }

    fn insert_block(&mut self, block: Block) -> Result<(), Self::Error> {
        self.blocks.insert(block.hash, block);
        Ok(())
    }

    fn insert_miss(&mut self, miss: Miss) -> Result<(), Self::Error> {
        self.misses.insert((miss.tx, miss.block), miss);
        Ok(())
    }

    fn insert_validator(&mut self, validator: Validator) -> Result<(), Self::Error> {
        self.validators.insert(validator.index, validator);
        Ok(())
    }
}
