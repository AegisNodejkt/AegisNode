use thiserror::Error;
use hyper::{Response, Body, StatusCode};
use serde_json::json;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Destination Error: {0}")]
    Destination(String),

    #[error("Internal Server Error: {0}")]
    Internal(String),

    #[error("Parsing Error: {0}")]
    Parsing(String),
    
    #[error("Configuration Error: {0}")]
    Config(String),
}

impl AppError {
    pub fn to_response(&self) -> Response<Body> {
        let (status, error_type, message) = match self {
            AppError::Destination(msg) => (StatusCode::BAD_GATEWAY, "destination_error", msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
            AppError::Parsing(msg) => (StatusCode::BAD_REQUEST, "parsing_error", msg),
            AppError::Config(msg) => (StatusCode::SERVICE_UNAVAILABLE, "config_error", msg),
        };

        let body = json!({
            "error": {
                "type": error_type,
                "message": message
            }
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap_or_else(|_| Response::new(Body::from("Internal Error building response")))
    }
}
