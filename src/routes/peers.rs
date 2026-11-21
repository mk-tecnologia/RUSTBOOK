use crate::{
    access::{resolve_book, Access},
    auth::middleware::AuthUser,
    error::ApiError,
    models::peer::*,
    pagination::Pagination,
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use std::collections::HashMap;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/ab/peers", get(get_peers).post(get_peers))
        .route("/api/ab/peer/add/{guid}", post(add_peer))
        .route("/api/ab/peer/update/{guid}", put(update_peer))
        .route("/api/ab/peer/{guid}", delete(delete_peers))
}

#[derive(Debug, Deserialize)]
struct PeersQuery {
    #[serde(default = "crate::pagination::default_page")]
    current: i64,
    #[serde(default = "crate::pagination::default_size", rename = "pageSize")]
    page_size: i64,
    #[serde(default)]
    ab: String,
    #[serde(default)]
    q: String,
    #[serde(default)]
    tag: String,
}

pub async fn peer_tags(
    db: &sqlx::SqlitePool,
    peers: &[Peer],
) -> Result<HashMap<i64, Vec<String>>, ApiError> {
    let mut tags: HashMap<i64, Vec<String>> = HashMap::new();
    for chunk in peers.chunks(500) {
        let mut query = sqlx::QueryBuilder::<sqlx::Sqlite>::new(
            "SELECT pt.peer_id, t.name FROM peer_tags pt JOIN tags t ON t.id = pt.tag_id WHERE pt.peer_id IN (");
        let mut ids = query.separated(", ");
        for peer in chunk {
            ids.push_bind(peer.id);
        }
        ids.push_unseparated(") ORDER BY t.name");
        let rows: Vec<(i64, String)> = query.build_query_as().fetch_all(db).await?;
        for (id, name) in rows {
            tags.entry(id).or_default().push(name);
        }
    }
    Ok(tags)
}

async fn get_peers(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(query): Query<PeersQuery>,
) -> Result<Json<PeersResponse>, ApiError> {
    let offset = Pagination {
        current: query.current,
        page_size: query.page_size,
    }
    .offset()?;
    if query.q.len() > 128 || query.tag.len() > 128 {
        return Err(ApiError::BadRequest("Filter is too long".into()));
    }
    let guid = resolve_book(&state.db, claims.user_id, &query.ab, Access::Read).await?;
    let filter = " FROM peers p WHERE ab_guid = ?
        AND (? = '' OR instr(lower(rustdesk_id || ' ' || alias || ' ' || hostname || ' ' || username), lower(?)) > 0)
        AND (? = '' OR EXISTS(SELECT 1 FROM peer_tags pt JOIN tags t ON t.id = pt.tag_id WHERE pt.peer_id = p.id AND t.name = ?))";
    let mut rows_query = sqlx::QueryBuilder::<sqlx::Sqlite>::new("SELECT p.*");
    rows_query
        .push(filter)
        .push(" ORDER BY rustdesk_id LIMIT ? OFFSET ?");
    let peers = rows_query
        .build_query_as::<Peer>()
        .bind(&guid)
        .bind(query.q.trim())
        .bind(query.q.trim())
        .bind(&query.tag)
        .bind(&query.tag)
        .bind(query.page_size)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;
    let mut count_query = sqlx::QueryBuilder::<sqlx::Sqlite>::new("SELECT COUNT(*)");
    count_query.push(filter);
    let total: i64 = count_query
        .build_query_scalar()
        .bind(&guid)
        .bind(query.q.trim())
        .bind(query.q.trim())
        .bind(&query.tag)
        .bind(&query.tag)
        .fetch_one(&state.db)
        .await?;
    let mut tags = peer_tags(&state.db, &peers).await?;
    Ok(Json(PeersResponse {
        data: peers
            .into_iter()
            .map(|peer| PeerPayload {
                tags: tags.remove(&peer.id).unwrap_or_default(),
                id: peer.rustdesk_id,
                hash: peer.hash,
                username: peer.username,
                hostname: peer.hostname,
                platform: peer.platform,
                alias: peer.alias,
                note: peer.note,
            })
            .collect(),
        total,
    }))
}

fn validate_id(id: &str) -> Result<(), ApiError> {
    if id.trim().is_empty() || id.len() > 128 {
        return Err(ApiError::BadRequest(
            "A peer ID of 1–128 bytes is required".into(),
        ));
    }
    Ok(())
}

async fn replace_tags(
    tx: &mut sqlx::SqliteConnection,
    guid: &str,
    peer_id: i64,
    tags: &[String],
) -> Result<(), ApiError> {
    if tags.len() > 100 {
        return Err(ApiError::BadRequest("At most 100 tags per peer".into()));
    }
    sqlx::query("DELETE FROM peer_tags WHERE peer_id = ?")
        .bind(peer_id)
        .execute(&mut *tx)
        .await?;
    for name in tags {
        let tag: Option<i64> =
            sqlx::query_scalar("SELECT id FROM tags WHERE ab_guid = ? AND name = ?")
                .bind(guid)
                .bind(name)
                .fetch_optional(&mut *tx)
                .await?;
        let tag = tag.ok_or_else(|| ApiError::BadRequest(format!("Unknown tag: {name}")))?;
        sqlx::query("INSERT OR IGNORE INTO peer_tags (peer_id, tag_id) VALUES (?, ?)")
            .bind(peer_id)
            .bind(tag)
            .execute(&mut *tx)
            .await?;
    }
    Ok(())
}

async fn add_peer(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
    Json(req): Json<AddPeerRequest>,
) -> Result<StatusOk, ApiError> {
    validate_id(&req.id)?;
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;
    let mut tx = state.db.begin().await?;
    let peer_id: i64 = sqlx::query_scalar(
        "INSERT INTO peers (ab_guid, rustdesk_id, hash, username, hostname, platform, alias, note)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(ab_guid, rustdesk_id) DO UPDATE SET
         hash = excluded.hash, username = excluded.username, hostname = excluded.hostname,
         platform = excluded.platform, alias = excluded.alias, note = excluded.note,
         updated_at = CURRENT_TIMESTAMP RETURNING id",
    )
    .bind(&guid)
    .bind(&req.id)
    .bind(req.hash)
    .bind(req.username)
    .bind(req.hostname)
    .bind(req.platform)
    .bind(req.alias)
    .bind(req.note)
    .fetch_one(&mut *tx)
    .await?;
    if let Some(tags) = req.tags {
        replace_tags(&mut tx, &guid, peer_id, &tags).await?;
    }
    tx.commit().await?;
    Ok(StatusOk)
}

// Empty 200 responses match the RustDesk client's action response parser.
pub struct StatusOk;
impl axum::response::IntoResponse for StatusOk {
    fn into_response(self) -> axum::response::Response {
        axum::http::StatusCode::OK.into_response()
    }
}

async fn update_peer(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
    Json(req): Json<UpdatePeerRequest>,
) -> Result<StatusOk, ApiError> {
    validate_id(&req.id)?;
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;
    let mut tx = state.db.begin().await?;
    let peer_id: Option<i64> = sqlx::query_scalar(
        "UPDATE peers SET hash = COALESCE(?, hash), username = COALESCE(?, username),
         hostname = COALESCE(?, hostname), platform = COALESCE(?, platform), alias = COALESCE(?, alias),
         note = COALESCE(?, note), updated_at = CURRENT_TIMESTAMP
         WHERE ab_guid = ? AND rustdesk_id = ? RETURNING id")
        .bind(req.hash).bind(req.username).bind(req.hostname).bind(req.platform).bind(req.alias).bind(req.note)
        .bind(&guid).bind(req.id).fetch_optional(&mut *tx).await?;
    let peer_id = peer_id.ok_or_else(|| ApiError::NotFound("Peer not found".into()))?;
    if let Some(tags) = req.tags {
        replace_tags(&mut tx, &guid, peer_id, &tags).await?;
    }
    tx.commit().await?;
    Ok(StatusOk)
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum DeleteRequest {
    List(Vec<String>),
    Object {
        #[serde(default)]
        ids: Vec<String>,
        #[serde(default)]
        id: Option<String>,
    },
}
impl DeleteRequest {
    pub fn ids(self) -> Result<Vec<String>, ApiError> {
        let ids = match self {
            Self::List(ids) => ids,
            Self::Object { mut ids, id } => {
                ids.extend(id);
                ids
            }
        };
        if ids.is_empty() || ids.len() > 100 {
            return Err(ApiError::BadRequest("Provide 1–100 identifiers".into()));
        }
        Ok(ids)
    }
}

async fn delete_peers(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(guid): Path<String>,
    Json(req): Json<DeleteRequest>,
) -> Result<StatusOk, ApiError> {
    let guid = resolve_book(&state.db, claims.user_id, &guid, Access::Write).await?;
    let ids = req.ids()?;
    let mut tx = state.db.begin().await?;
    for id in ids {
        sqlx::query("DELETE FROM peers WHERE ab_guid = ? AND rustdesk_id = ?")
            .bind(&guid)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(StatusOk)
}
