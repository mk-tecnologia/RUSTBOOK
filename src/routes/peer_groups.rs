use crate::{
    access::{resolve_book, Access},
    auth::middleware::AuthUser,
    error::ApiError,
    routes::peers::StatusOk,
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/ab/peer-groups/{guid}", get(list).post(create))
        .route(
            "/api/ab/peer-groups/{guid}/{id}",
            put(update).delete(remove),
        )
}

#[derive(Default, Deserialize)]
struct Filter {
    #[serde(default)]
    q: String,
    #[serde(default)]
    tag: String,
}

#[derive(Deserialize)]
struct GroupRequest {
    name: String,
    #[serde(default)]
    note: String,
}

fn validate(req: &GroupRequest) -> Result<(), ApiError> {
    if req.name.trim().is_empty() || req.name.trim().len() > 128 || req.note.len() > 2048 {
        return Err(ApiError::BadRequest(
            "Group name must contain 1–128 bytes; note at most 2048 bytes".into(),
        ));
    }
    Ok(())
}

fn write_error(error: sqlx::Error) -> ApiError {
    match error {
        sqlx::Error::Database(ref db) if db.is_unique_violation() => ApiError::Conflict(
            "A device group with this name already exists in this address book".into(),
        ),
        _ => ApiError::Internal(error.to_string()),
    }
}

async fn list(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
    Query(filter): Query<Filter>,
) -> Result<Json<Value>, ApiError> {
    if filter.q.len() > 128 || filter.tag.len() > 128 {
        return Err(ApiError::BadRequest("Filter is too long".into()));
    }
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Read).await?;
    let mut tx = state.db.begin().await?;
    let counts: Vec<(Option<i64>, i64)> = sqlx::query_as(
        "SELECT group_id, COUNT(*) FROM peers p WHERE ab_guid = ?
         AND (? = '' OR instr(lower(rustdesk_id || ' ' || alias || ' ' || hostname || ' ' || username), lower(?)) > 0)
         AND (? = '' OR EXISTS(SELECT 1 FROM peer_tags pt JOIN tags t ON t.id = pt.tag_id WHERE pt.peer_id = p.id AND t.name = ?))
         GROUP BY group_id")
        .bind(&guid).bind(filter.q.trim()).bind(filter.q.trim()).bind(&filter.tag).bind(&filter.tag)
        .fetch_all(&mut *tx).await?;
    let total: i64 = counts.iter().map(|(_, count)| count).sum();
    let counts: HashMap<Option<i64>, i64> = counts.into_iter().collect();
    let groups: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT id, name, note FROM peer_groups WHERE ab_guid = ? ORDER BY name COLLATE NOCASE, id",
    )
    .bind(&guid)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let data: Vec<Value> = groups.into_iter().map(|(id, name, note)| json!({
        "id": id, "name": name, "note": note, "peer_count": counts.get(&Some(id)).copied().unwrap_or(0)
    })).collect();
    Ok(Json(
        json!({"data": data, "total": total, "ungrouped_count": counts.get(&None).copied().unwrap_or(0)}),
    ))
}

async fn create(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
    Json(req): Json<GroupRequest>,
) -> Result<Json<Value>, ApiError> {
    validate(&req)?;
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO peer_groups(ab_guid, name, note) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(guid)
    .bind(req.name.trim())
    .bind(req.note)
    .fetch_one(&state.db)
    .await
    .map_err(write_error)?;
    Ok(Json(json!({"id": id})))
}

async fn update(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path((guid, id)): Path<(String, i64)>,
    Json(req): Json<GroupRequest>,
) -> Result<StatusOk, ApiError> {
    validate(&req)?;
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;
    let result =
        sqlx::query("UPDATE peer_groups SET name = ?, note = ? WHERE id = ? AND ab_guid = ?")
            .bind(req.name.trim())
            .bind(req.note)
            .bind(id)
            .bind(guid)
            .execute(&state.db)
            .await
            .map_err(write_error)?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Device group not found".into()));
    }
    Ok(StatusOk)
}

async fn remove(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path((guid, id)): Path<(String, i64)>,
) -> Result<StatusOk, ApiError> {
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;
    // ON DELETE SET NULL keeps peers and their tags, returning them to Ungrouped.
    let result = sqlx::query("DELETE FROM peer_groups WHERE id = ? AND ab_guid = ?")
        .bind(id)
        .bind(guid)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Device group not found".into()));
    }
    Ok(StatusOk)
}
