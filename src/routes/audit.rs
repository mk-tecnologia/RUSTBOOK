use crate::{auth::middleware::AuthUser, error::ApiError, pagination::Pagination, state::AppState};
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use serde_json::{json, Value};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/audit-log", get(list))
}
#[derive(sqlx::FromRow, Serialize)]
struct Entry {
    id: i64,
    user_id: Option<i64>,
    action: String,
    rustdesk_id: String,
    peer_id: String,
    ip: String,
    note: String,
    created_at: String,
}
async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(page): Query<Pagination>,
) -> Result<Json<Value>, ApiError> {
    if !claims.is_admin {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }
    let offset = page.offset()?;
    let data = sqlx::query_as::<_, Entry>(
        "SELECT * FROM audit_log ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?",
    )
    .bind(page.page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&state.db)
        .await?;
    Ok(Json(json!({ "data": data, "total": total })))
}
