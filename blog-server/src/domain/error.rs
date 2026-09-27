use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlogError {
    #[error("user not found")]
    UserNotFound,
    #[error("user already exists")]
    UserAlreadyExists,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("post not found")]
    PostNotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("unauthorized")]
    Unauthorized,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("internal server error")]
    Internal,
}

impl ResponseError for BlogError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UserAlreadyExists => StatusCode::CONFLICT,
            Self::UserNotFound | Self::PostNotFound => StatusCode::NOT_FOUND,
            Self::InvalidCredentials | Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .json(serde_json::json!({ "error": self.to_string() }))
    }
}

impl From<sqlx::Error> for BlogError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error, "database operation failed");
        Self::Internal
    }
}
