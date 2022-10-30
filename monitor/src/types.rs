use std::fmt;

pub use ethers::types::{
    Address, Block, Transaction, TxHash, TxpoolContent, TxpoolTransaction, H256,
};
pub type Timestamp = u64;

/// ChronologyError is returned if events are reported in wrong order.
#[derive(Debug, PartialEq)]
pub struct ChronologyError;

impl fmt::Display for ChronologyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "observations added in non-chronological order")
    }
}

/// MissingBlockFieldError is returned if a block argument is missing a required field (e.g. a
/// hash).
#[derive(Debug, PartialEq)]
pub struct MissingBlockFieldError {
    field: String,
}

impl MissingBlockFieldError {
    pub fn new(field: String) -> Self {
        MissingBlockFieldError { field }
    }
}

impl fmt::Display for MissingBlockFieldError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "block argument is missing required field {}", self.field)
    }
}
