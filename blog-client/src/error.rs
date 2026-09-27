use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlogClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),
    #[error("post not found")]
    NotFound,
    #[error("authentication required")]
    Unauthorized,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("server error: {0}")]
    Server(String),
}

pub fn status_error(status: reqwest::StatusCode, body: &str) -> BlogClientError {
    match status {
        reqwest::StatusCode::NOT_FOUND => BlogClientError::NotFound,
        reqwest::StatusCode::UNAUTHORIZED => BlogClientError::Unauthorized,
        reqwest::StatusCode::BAD_REQUEST => BlogClientError::InvalidRequest(body.to_string()),
        _ => BlogClientError::Server(format!("HTTP {status}: {body}")),
    }
}
