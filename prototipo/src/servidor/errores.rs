use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
    pub fn bad_request(code: &'static str, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, msg)
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg)
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "FORBIDDEN", msg)
    }
    pub fn not_found(code: &'static str, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, msg)
    }
    pub fn conflict(code: &'static str, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, msg)
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({
            "success": false,
            "data": null,
            "error": { "code": self.code, "message": self.message }
        });
        (self.status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

pub fn ok_json<T: serde::Serialize>(data: T) -> Response {
    let body = json!({ "success": true, "data": data, "error": null });
    (StatusCode::OK, Json(body)).into_response()
}

pub fn created_json<T: serde::Serialize>(data: T) -> Response {
    let body = json!({ "success": true, "data": data, "error": null });
    (StatusCode::CREATED, Json(body)).into_response()
}
