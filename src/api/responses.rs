use chrono::{naive::serde::ts_seconds_option, NaiveDateTime};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ItemizedResponse<T> {
    items: Vec<T>,
    #[serde(with = "ts_seconds_option")]
    from: Option<NaiveDateTime>,
    #[serde(with = "ts_seconds_option")]
    to: Option<NaiveDateTime>,
}

impl<T> ItemizedResponse<T> {
    pub fn new(
        items: Vec<T>,
        data_range: Option<(NaiveDateTime, NaiveDateTime)>,
        query_from: Option<NaiveDateTime>,
        query_to: Option<NaiveDateTime>,
    ) -> Self {
        let (from, to) = if let Some((from, to)) = data_range {
            (Some(from), Some(to))
        } else {
            (query_from, query_to)
        };
        Self { items, from, to }
    }
}

pub trait ResponseItem {
    fn get_ref_time(&self) -> NaiveDateTime;
}

pub fn get_time_range<T: ResponseItem>(misses: &Vec<T>) -> Option<(NaiveDateTime, NaiveDateTime)> {
    if misses.is_empty() {
        None
    } else {
        Some((
            misses[0].get_ref_time(),
            misses[misses.len() - 1].get_ref_time(),
        ))
    }
}
