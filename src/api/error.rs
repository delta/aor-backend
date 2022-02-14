use actix_web::error::ErrorInternalServerError;
use actix_web::ResponseError;
use derive_more::Display;
use log;
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum AuthError {
    SessionError,
    UnVerifiedError,
}

impl ResponseError for AuthError {
    fn error_response(&self) -> actix_web::HttpResponse {
        let response_body = match self {
            AuthError::SessionError => "Session Error. Please login again.",
            AuthError::UnVerifiedError => "Please verify your account.",
        };
        actix_web::HttpResponse::Unauthorized().body(response_body)
    }
}

pub fn handle_error(err: Box<dyn std::error::Error>) -> actix_web::Error {
    log::error!("{}", err);
    ErrorInternalServerError("Internal Server Error")
}
