use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::auth::jwt::{create_token, validate_token};
use crate::auth::middleware::AuthUser;
use crate::auth::password::verify_async;
use crate::error::ApiError;
use crate::models::user::{LoginRequest, LoginResponse, UserPayload};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/login-options", get(login_options))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/currentUser", get(current_user).post(current_user))
}

// Password authentication only; the client expects a list of OIDC providers.
async fn login_options() -> Json<Vec<String>> {
    Json(Vec::new())
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    if req.username.is_empty() || req.username.len() > 128 || req.password.len() > 1024 {
        return Err(ApiError::BadRequest("Invalid credentials format".into()));
    }
    crate::auth::throttle::check(&req.username)?;
    let user = sqlx::query_as::<_, crate::models::user::User>(
        "SELECT * FROM users WHERE username = ? AND status = 1",
    )
    .bind(&req.username)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::Unauthorized("Invalid username or password".to_string()))?;

    if !verify_async(req.password, user.password_hash.clone()).await? {
        return Err(ApiError::Unauthorized(
            "Invalid username or password".to_string(),
        ));
    }

    let token = create_token(
        &user.username,
        user.id,
        user.is_admin,
        &state.config.jwt_secret,
        state.config.token_expiry_hours,
    )?;

    let claims = validate_token(&token, &state.config.jwt_secret)?;
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM sessions WHERE expires_at <= unixepoch()")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO sessions (id, user_id, expires_at) SELECT ?, id, ? FROM users WHERE id = ? AND status = 1 AND password_hash = ?")
        .bind(&claims.jti).bind(claims.exp as i64).bind(user.id).bind(&user.password_hash)
        .execute(&mut *tx).await?
        .rows_affected().eq(&1).then_some(())
        .ok_or_else(|| ApiError::Unauthorized("Credentials changed; sign in again".into()))?;
    sqlx::query("INSERT INTO audit_log (user_id, action, rustdesk_id) VALUES (?, 'login', ?)")
        .bind(user.id)
        .bind(&req.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "access_token".to_string(),
        user: UserPayload {
            name: user.username,
            display_name: user.name,
            email: user.email,
            is_admin: user.is_admin,
            note: String::new(),
        },
    }))
}

async fn logout(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(&claims.jti)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({})))
}

async fn current_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    let user = sqlx::query_as::<_, crate::models::user::User>("SELECT * FROM users WHERE id = ?")
        .bind(claims.user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok(Json(json!({
        "name": user.username,
        "display_name": user.name,
        "email": user.email,
        "is_admin": user.is_admin,
        "note": ""
    })))
}
