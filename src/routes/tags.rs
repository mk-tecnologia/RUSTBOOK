use axum::{
    extract::{Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};

use crate::access::{resolve_book, Access};
use crate::auth::middleware::AuthUser;
use crate::error::ApiError;
use crate::models::tag::*;
use crate::routes::peers::{DeleteRequest, StatusOk};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/ab/tags/{guid}", get(get_tags).post(get_tags_client))
        .route("/api/ab/tag/add/{guid}", post(add_tag))
        .route("/api/ab/tag/rename/{guid}", put(rename_tag))
        .route("/api/ab/tag/update/{guid}", put(update_tag_color))
        .route("/api/ab/tag/{guid}", delete(delete_tags))
}

async fn get_tags(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
) -> Result<Json<TagsResponse>, ApiError> {
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Read).await?;

    let tags = sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE ab_guid = ? ORDER BY name")
        .bind(&guid)
        .fetch_all(&state.db)
        .await?;

    let total = tags.len() as i64;
    let data: Vec<TagPayload> = tags
        .into_iter()
        .map(|t| TagPayload {
            name: t.name,
            color: t.color,
        })
        .collect();

    Ok(Json(TagsResponse { data, total }))
}

/// Native clients expect a JSON array; GET retains the web envelope.
async fn get_tags_client(
    state: State<AppState>,
    user: AuthUser,
    guid: Path<String>,
) -> Result<Json<Vec<TagPayload>>, ApiError> {
    let Json(response) = get_tags(state, user, guid).await?;
    Ok(Json(response.data))
}

async fn add_tag(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
    Json(req): Json<AddTagRequest>,
) -> Result<StatusOk, ApiError> {
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;

    validate_name(&req.name)?;
    sqlx::query("INSERT OR IGNORE INTO tags (ab_guid, name, color) VALUES (?, ?, ?)")
        .bind(&guid)
        .bind(&req.name)
        .bind(req.color)
        .execute(&state.db)
        .await?;

    Ok(StatusOk)
}

async fn rename_tag(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
    Json(req): Json<RenameTagRequest>,
) -> Result<StatusOk, ApiError> {
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;

    validate_name(&req.new)?;
    sqlx::query("UPDATE tags SET name = ? WHERE ab_guid = ? AND name = ?")
        .bind(&req.new)
        .bind(&guid)
        .bind(&req.old)
        .execute(&state.db)
        .await?;

    Ok(StatusOk)
}

async fn update_tag_color(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
    Json(req): Json<UpdateTagColorRequest>,
) -> Result<StatusOk, ApiError> {
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;

    sqlx::query("UPDATE tags SET color = ? WHERE ab_guid = ? AND name = ?")
        .bind(req.color)
        .bind(&guid)
        .bind(&req.name)
        .execute(&state.db)
        .await?;

    Ok(StatusOk)
}

async fn delete_tags(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
    Json(req): Json<DeleteTags>,
) -> Result<StatusOk, ApiError> {
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;

    let names = match req {
        DeleteTags::List(names) => names,
        DeleteTags::Object { names } => names,
    };
    let names = DeleteRequest::List(names).ids()?;
    let mut tx = state.db.begin().await?;

    for name in &names {
        // Remove peer_tags associations first
        sqlx::query(
            "DELETE FROM peer_tags WHERE tag_id IN (SELECT id FROM tags WHERE ab_guid = ? AND name = ?)",
        )
        .bind(&guid)
        .bind(name)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM tags WHERE ab_guid = ? AND name = ?")
            .bind(&guid)
            .bind(name)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(StatusOk)
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum DeleteTags {
    List(Vec<String>),
    Object { names: Vec<String> },
}

fn validate_name(name: &str) -> Result<(), ApiError> {
    if name.trim().is_empty() || name.len() > 128 {
        return Err(ApiError::BadRequest(
            "Tag name must contain 1–128 bytes".into(),
        ));
    }
    Ok(())
}
