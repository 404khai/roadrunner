use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ErrorBody {
    pub code: String,
    pub message: String,
}
#[derive(Debug)]
pub(crate) struct ApiError {
    pub status: StatusCode,
    pub body: ErrorBody,
}
impl ApiError {
    // Own arbitrary domain errors while erasing their concrete type at the HTTP boundary.
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(status: StatusCode, code: &str, message: impl ToString) -> Self {
        Self {
            status,
            body: ErrorBody {
                code: code.into(),
                message: message.to_string(),
            },
        }
    }
    pub fn invalid(message: impl ToString) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid_input", message)
    }
    pub fn conflict(message: impl ToString) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }
    pub fn missing() -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            "not_found",
            "resource not found in the living namespace",
        )
    }
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "request could not be completed",
        )
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}
