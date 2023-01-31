use serde::Serialize;

use super::{miss_time_tuple::serde_miss_time_tuple, MissTimeTuple};

#[derive(Debug, Serialize)]
pub struct ItemizedResponse<T> {
    items: Vec<T>,
    complete: bool,
    #[serde(with = "serde_miss_time_tuple")]
    from: MissTimeTuple,
    #[serde(with = "serde_miss_time_tuple")]
    to: MissTimeTuple,
}

impl<T> ItemizedResponse<T> {
    pub fn new(
        items: Vec<T>,
        complete: bool,
        query_from: MissTimeTuple,
        query_to: MissTimeTuple,
        data_to: Option<MissTimeTuple>,
    ) -> Self {
        let to = if complete {
            query_to
        } else {
            data_to.unwrap_or(query_to)
        };
        Self {
            items,
            complete,
            from: query_from,
            to,
        }
    }
}

pub trait ResponseItem {
    fn get_source_miss_time_tuple(&self) -> MissTimeTuple;
}

pub fn get_last_source_miss_time_tuple<T: ResponseItem>(items: &Vec<T>) -> Option<MissTimeTuple> {
    if items.is_empty() {
        None
    } else {
        Some(items[items.len() - 1].get_source_miss_time_tuple())
    }
}
