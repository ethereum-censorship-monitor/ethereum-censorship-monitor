use chrono::NaiveDateTime;
use serde::Serialize;

use super::{miss_range_bound::serde::miss_range_bound, MissRangeBound};

#[derive(Debug, Serialize)]
pub struct ItemizedResponse<T> {
    items: Vec<T>,
    complete: bool,
    #[serde(with = "miss_range_bound")]
    from: MissRangeBound,
    #[serde(with = "miss_range_bound")]
    to: MissRangeBound,
}

impl<T> ItemizedResponse<T> {
    pub fn new(
        items: Vec<T>,
        complete: bool,
        query_from: MissRangeBound,
        query_to: NaiveDateTime,
        data_to: Option<MissRangeBound>,
    ) -> Self {
        #[allow(clippy::unnecessary_unwrap)]
        let to = if complete || data_to.is_none() {
            MissRangeBound {
                proposal_time: query_to,
                offset: None,
            }
        } else {
            data_to.unwrap()
        };
        Self {
            items,
            complete,
            from: query_from,
            to,
        }
    }
}
