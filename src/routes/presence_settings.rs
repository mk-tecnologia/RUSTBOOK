use crate::{auth::middleware::AuthUser, error::ApiError, state::AppState};
use axum::{extract::State, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/admin/presence", get(settings).put(save).delete(reset))
}
fn admin(claims: &crate::auth::jwt::Claims) -> Result<(), ApiError> {
    if !claims.is_admin {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }
    Ok(())
}
async fn describe(endpoint: &str, saved: bool) -> Value {
    let status = if endpoint.is_empty() {
        "not_configured"
    } else if crate::presence::query(endpoint, &["rustbook-presence-check".into()])
        .await
        .is_ok()
    {
        "connected"
    } else {
        "unreachable"
    };
    json!({"server": endpoint, "source": if saved { "saved" } else { "deployment" }, "status": status})
}
async fn settings(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    admin(&claims)?;
    let (endpoint, saved) = crate::presence::server(&state).await?;
    Ok(Json(describe(&endpoint, saved).await))
}
#[derive(Deserialize)]
struct Settings {
    server: String,
}
async fn save(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<Settings>,
) -> Result<Json<Value>, ApiError> {
    admin(&claims)?;
    let endpoint = req.server.trim();
    let mut config = state.config.clone();
    config.rendezvous_server = endpoint.into();
    config.validate().map_err(ApiError::BadRequest)?;
    let report = describe(endpoint, true).await;
    if report["status"] == "unreachable" {
        return Err(ApiError::BadRequest("Unable to query the ID server. Check its NAT TCP port (usually 21115) and connectivity from this server. The previous configuration was kept.".into()));
    }
    sqlx::query("INSERT INTO server_settings(key, value) VALUES ('rendezvous_server', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
        .bind(endpoint).execute(&state.db).await?;
    Ok(Json(report))
}
async fn reset(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    admin(&claims)?;
    sqlx::query("DELETE FROM server_settings WHERE key = 'rendezvous_server'")
        .execute(&state.db)
        .await?;
    Ok(Json(describe(&state.config.rendezvous_server, false).await))
}
