use std::fmt;

pub use ethers::types::{
    Address, Block, Transaction, TxHash, TxpoolContent, TxpoolTransaction, H256,
};
pub type Timestamp = u64;

/// ChronologyError is returned when events are reported in wrong order.
#[derive(Debug, PartialEq)]
pub struct ChronologyError;

impl fmt::Display for ChronologyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "observations added in non-chronological order")
    }
}
