use chrono::{naive::serde::ts_seconds, NaiveDateTime};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ItemizedResponse<T> {
    items: Vec<T>,
    complete: bool,
    #[serde(with = "ts_seconds")]
    from: NaiveDateTime,
    #[serde(with = "ts_seconds")]
    to: NaiveDateTime,
}

impl<T> ItemizedResponse<T> {
    pub fn new(
        items: Vec<T>,
        complete: bool,
        query_from: NaiveDateTime,
        query_to: NaiveDateTime,
        data_to: Option<NaiveDateTime>,
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
    fn get_ref_time(&self) -> NaiveDateTime;
}

pub fn get_last_ref_time<T: ResponseItem>(items: &Vec<T>) -> Option<NaiveDateTime> {
    if items.is_empty() {
        None
    } else {
        Some(items[items.len() - 1].get_ref_time())
    }
}
