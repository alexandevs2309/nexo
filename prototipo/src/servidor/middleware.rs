use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;

use crate::servidor::auth;
use crate::servidor::errores::ApiError;
use crate::servidor::rutas::AppState;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: uuid::Uuid,
    pub role: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("falta Authorization"))?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::unauthorized("esquema Bearer requerido"))?;
        let (uid, role) = auth::validar_token(&state.pool, token)
            .await
            .map_err(ApiError::unauthorized)?;
        Ok(AuthUser { user_id: uid, role })
    }
}

pub fn require_role(user: &AuthUser, roles: &[&str]) -> Result<(), ApiError> {
    if roles.contains(&user.role.as_str()) {
        Ok(())
    } else {
        Err(ApiError::forbidden("permisos insuficientes"))
    }
}

#[allow(dead_code)]
pub fn _status_ok() -> StatusCode {
    StatusCode::OK
}
