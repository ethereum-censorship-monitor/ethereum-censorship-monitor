use actix_web::{http::StatusCode, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("internal error")]
pub struct InternalError {}

impl ResponseError for InternalError {}

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("Query parameter {parameter} is out of range")]
    ParameterOutOfRange { parameter: String },
}

impl ResponseError for RequestError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}
