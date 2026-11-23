use crate::{auth::middleware::AuthUser, error::ApiError, state::AppState};
use axum::{
    extract::State,
    routing::{get, put},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/connection-settings", get(read))
        .route("/api/admin/connection-settings", put(save))
}
#[derive(Default, Deserialize, Serialize)]
pub struct ConnectionSettings {
    server: String,
    public_key: String,
}
async fn read(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    let saved: Option<String> =
        sqlx::query_scalar("SELECT value FROM server_settings WHERE key = 'native_connection'")
            .fetch_optional(&state.db)
            .await?;
    let profile: ConnectionSettings = saved
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(|error| ApiError::Internal(error.to_string()))?
        .unwrap_or_default();
    Ok(Json(
        json!({"server":profile.server,"public_key":profile.public_key,"can_edit":claims.is_admin}),
    ))
}
async fn save(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<ConnectionSettings>,
) -> Result<Json<Value>, ApiError> {
    if !claims.is_admin {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }
    let profile = ConnectionSettings {
        server: req.server.trim().into(),
        public_key: req.public_key.trim().into(),
    };
    if !profile.server.is_empty() || !profile.public_key.is_empty() {
        let mut config = state.config.clone();
        config.rendezvous_server = profile.server.clone();
        if profile.server.is_empty() || config.validate().is_err() {
            return Err(ApiError::BadRequest(
                "Use an explicit ID server hostname:port, normally hostname:21116".into(),
            ));
        }
        if STANDARD
            .decode(&profile.public_key)
            .map_or(true, |key| key.len() != 32)
        {
            return Err(ApiError::BadRequest("Provide the hbbs public Key (32 bytes in Base64), never a private key or account token".into()));
        }
    }
    let mut tx = state.db.begin().await?;
    sqlx::query("INSERT INTO server_settings(key,value) VALUES ('native_connection', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(serde_json::to_string(&profile).map_err(|error| ApiError::Internal(error.to_string()))?).execute(&mut *tx).await?;
    crate::routes::server_settings::bump_revision(&mut tx).await?;
    tx.commit().await?;
    Ok(Json(
        json!({"server":profile.server,"public_key":profile.public_key,"can_edit":true}),
    ))
}
