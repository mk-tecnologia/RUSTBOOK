use axum::{extract::FromRequestParts, http::request::Parts};

use crate::auth::jwt::{validate_token, Claims};
use crate::error::ApiError;
use crate::state::AppState;

/// Extractor that validates the Bearer token and provides user claims.
pub struct AuthUser(pub Claims);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::Unauthorized("Missing authorization header".to_string()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| ApiError::Unauthorized("Invalid authorization format".to_string()))?;

        let mut claims = validate_token(token, &state.config.jwt_secret)
            .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

        let current: Option<(String, bool)> = sqlx::query_as(
            "SELECT u.username, u.is_admin FROM users u JOIN sessions s ON s.user_id = u.id
             WHERE u.id = ? AND u.status = 1 AND s.id = ? AND s.expires_at > unixepoch()",
        )
        .bind(claims.user_id)
        .bind(&claims.jti)
        .fetch_optional(&state.db)
        .await?;
        let (username, is_admin) = current
            .ok_or_else(|| ApiError::Unauthorized("Session revoked or user disabled".into()))?;
        claims.sub = username;
        claims.is_admin = is_admin;
        Ok(AuthUser(claims))
    }
}
