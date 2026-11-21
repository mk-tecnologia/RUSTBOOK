use crate::{auth::middleware::AuthUser, error::ApiError, pagination::Pagination, state::AppState};
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/peers", get(list))
        .route("/api/device-group/accessible", get(accessible_groups))
}

#[derive(Deserialize)]
struct DevicesQuery {
    #[serde(default = "crate::pagination::default_page")]
    current: i64,
    #[serde(default = "crate::pagination::default_size", rename = "pageSize")]
    page_size: i64,
}

#[derive(sqlx::FromRow)]
struct Device {
    rustdesk_id: String,
    hostname: String,
    os: String,
    version: String,
    status: i32,
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(query): Query<DevicesQuery>,
) -> Result<Json<Value>, ApiError> {
    let offset = Pagination {
        current: query.current,
        page_size: query.page_size,
    }
    .offset()?;
    // No ownership or ACL exists in the upstream devices schema. Deny visibility
    // to ordinary accounts rather than exposing the global device inventory.
    if !claims.is_admin {
        return Ok(Json(json!({"data": [], "total": 0})));
    }
    // Future/invalid timestamps must not make a device appear online indefinitely.
    let rows: Vec<Device> = sqlx::query_as(
        "SELECT rustdesk_id, hostname, CASE WHEN os = '' THEN platform ELSE os END AS os, version,
         CASE WHEN unixepoch(last_online) BETWEEN unixepoch() - 120 AND unixepoch() THEN 1 ELSE 0 END AS status
         FROM devices ORDER BY rustdesk_id LIMIT ? OFFSET ?")
        .bind(query.page_size).bind(offset).fetch_all(&state.db).await?;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM devices")
        .fetch_one(&state.db)
        .await?;
    let data: Vec<Value> = rows.into_iter().map(|device| json!({
        "id": device.rustdesk_id,
        "info": {"device_name": device.hostname, "os": device.os, "version": device.version, "username": ""},
        "status": device.status, "user": "", "user_name": "", "device_group_name": "", "note": ""
    })).collect();
    Ok(Json(json!({"data": data, "total": total})))
}

async fn accessible_groups(
    AuthUser(_claims): AuthUser,
    Query(page): Query<Pagination>,
) -> Result<Json<Value>, ApiError> {
    page.offset()?;
    // User groups are not device groups. Preserve the protocol without pretending
    // that the upstream schema implements device grouping or authorization.
    Ok(Json(json!({"data": [], "total": 0})))
}
