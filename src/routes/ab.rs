use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::middleware::AuthUser;
use crate::error::ApiError;
use crate::models::address_book::*;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/ab", get(get_ab_legacy).post(update_ab_legacy))
        .route(
            "/api/ab/personal",
            get(get_personal).post(get_personal_client),
        )
        .route(
            "/api/ab/shared/profiles",
            get(get_shared_profiles).post(get_shared_profiles),
        )
        .route(
            "/api/ab/settings",
            get(get_ab_settings).post(get_ab_settings),
        )
}

/// Ensure the user has a personal address book, creating one if needed.
/// Returns the personal AB's guid.
async fn ensure_personal_ab(
    db: &sqlx::SqlitePool,
    user_id: i64,
    username: &str,
) -> Result<String, ApiError> {
    sqlx::query("INSERT INTO address_books (guid, name, owner_id, is_personal) VALUES (?, 'Personal', ?, TRUE) ON CONFLICT DO NOTHING")
        .bind(Uuid::new_v4().to_string()).bind(user_id).execute(db).await?;
    let guid = sqlx::query_scalar(
        "SELECT guid FROM address_books WHERE owner_id = ? AND is_personal = TRUE",
    )
    .bind(user_id)
    .fetch_one(db)
    .await?;
    tracing::debug!("Resolved personal address book for {username}");
    Ok(guid)
}

/// GET /api/ab/personal — returns the user's personal address book profile.
async fn get_personal(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<AbPersonalResponse>, ApiError> {
    let guid = ensure_personal_ab(&state.db, claims.user_id, &claims.sub).await?;

    Ok(Json(AbPersonalResponse {
        data: AbProfile {
            guid,
            name: "Personal".to_string(),
            owner: claims.sub,
            rule: 3, // full control
            note: String::new(),
        },
    }))
}

/// Native clients use a bodyless POST and read the root profile.
/// GET retains the web console envelope.
async fn get_personal_client(
    state: State<AppState>,
    user: AuthUser,
) -> Result<Json<AbProfile>, ApiError> {
    let Json(response) = get_personal(state, user).await?;
    Ok(Json(response.data))
}

/// GET /api/ab — legacy endpoint, returns the entire address book as a JSON string.
async fn get_ab_legacy(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<LegacyAbResponse>, ApiError> {
    let guid = ensure_personal_ab(&state.db, claims.user_id, &claims.sub).await?;

    // Fetch all peers
    let peers = sqlx::query_as::<_, crate::models::peer::Peer>(
        "SELECT * FROM peers WHERE ab_guid = ? ORDER BY rustdesk_id",
    )
    .bind(&guid)
    .fetch_all(&state.db)
    .await?;

    // Fetch all tags
    let tags = sqlx::query_as::<_, crate::models::tag::Tag>(
        "SELECT * FROM tags WHERE ab_guid = ? ORDER BY name",
    )
    .bind(&guid)
    .fetch_all(&state.db)
    .await?;

    // Build tag names list
    let tag_names: Vec<String> = tags.iter().map(|t| t.name.clone()).collect();

    // Build tag_colors map as a JSON string
    let mut tag_colors_map = serde_json::Map::new();
    for tag in &tags {
        tag_colors_map.insert(tag.name.clone(), json!(tag.color));
    }
    let tag_colors_str = serde_json::to_string(&tag_colors_map).unwrap_or_default();

    let mut all_tags = crate::routes::peers::peer_tags(&state.db, &peers).await?;
    let mut legacy_peers = Vec::new();
    for peer in &peers {
        let peer_tags = all_tags.remove(&peer.id).unwrap_or_default();

        legacy_peers.push(LegacyPeer {
            id: peer.rustdesk_id.clone(),
            hash: peer.hash.clone(),
            username: peer.username.clone(),
            hostname: peer.hostname.clone(),
            platform: peer.platform.clone(),
            alias: peer.alias.clone(),
            tags: peer_tags,
        });
    }

    let ab_data = LegacyAbData {
        tags: tag_names,
        peers: legacy_peers,
        tag_colors: tag_colors_str,
    };

    let data_str =
        serde_json::to_string(&ab_data).map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(LegacyAbResponse { data: data_str }))
}

/// POST /api/ab — legacy endpoint, replaces the entire address book from a JSON string.
async fn update_ab_legacy(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<LegacyAbUpdateRequest>,
) -> Result<crate::routes::peers::StatusOk, ApiError> {
    let guid = ensure_personal_ab(&state.db, claims.user_id, &claims.sub).await?;

    let ab_data: LegacyAbData = serde_json::from_str(&req.data)
        .map_err(|e| ApiError::BadRequest(format!("Invalid address book data: {}", e)))?;

    // Parse tag_colors
    let tag_colors: std::collections::HashMap<String, i64> =
        serde_json::from_str(&ab_data.tag_colors).unwrap_or_default();

    let mut tx = state.db.begin().await?;
    // Delete existing data and replace atomically
    // Delete peer_tags first (foreign key), then peers, then tags
    sqlx::query("DELETE FROM peer_tags WHERE peer_id IN (SELECT id FROM peers WHERE ab_guid = ?)")
        .bind(&guid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM peers WHERE ab_guid = ?")
        .bind(&guid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM tags WHERE ab_guid = ?")
        .bind(&guid)
        .execute(&mut *tx)
        .await?;

    // Insert tags
    for tag_name in &ab_data.tags {
        let color = tag_colors.get(tag_name).copied().unwrap_or(4278190080);
        sqlx::query("INSERT INTO tags (ab_guid, name, color) VALUES (?, ?, ?)")
            .bind(&guid)
            .bind(tag_name)
            .bind(color)
            .execute(&mut *tx)
            .await?;
    }

    // Insert peers and their tag associations
    for peer in &ab_data.peers {
        let result = sqlx::query(
            "INSERT INTO peers (ab_guid, rustdesk_id, hash, username, hostname, platform, alias) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&guid)
        .bind(&peer.id)
        .bind(&peer.hash)
        .bind(&peer.username)
        .bind(&peer.hostname)
        .bind(&peer.platform)
        .bind(&peer.alias)
        .execute(&mut *tx)
        .await?;

        let peer_id = result.last_insert_rowid();

        for tag_name in &peer.tags {
            let tag_id: Option<i64> =
                sqlx::query_scalar("SELECT id FROM tags WHERE ab_guid = ? AND name = ?")
                    .bind(&guid)
                    .bind(tag_name)
                    .fetch_optional(&mut *tx)
                    .await?;

            if let Some(tag_id) = tag_id {
                sqlx::query("INSERT OR IGNORE INTO peer_tags (peer_id, tag_id) VALUES (?, ?)")
                    .bind(peer_id)
                    .bind(tag_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }

    tx.commit().await?;
    Ok(crate::routes::peers::StatusOk)
}

/// GET /api/ab/shared/profiles — returns shared address books the user has access to.
async fn get_shared_profiles(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<AbSharedProfilesResponse>, ApiError> {
    let shared: Vec<(String, String, String, i32)> = sqlx::query_as(
        "SELECT ab.guid, ab.name, u.username, MAX(s.rule)
         FROM address_books ab JOIN users u ON u.id = ab.owner_id
         JOIN ab_shares s ON s.ab_guid = ab.guid
         WHERE (s.user_id = ? OR s.group_id IN (SELECT group_id FROM user_groups WHERE user_id = ?))
         AND s.rule BETWEEN 1 AND 3 AND ab.is_personal = FALSE
         GROUP BY ab.guid ORDER BY ab.name, ab.guid",
    )
    .bind(claims.user_id)
    .bind(claims.user_id)
    .fetch_all(&state.db)
    .await?;
    let profiles: Vec<AbProfile> = shared
        .into_iter()
        .map(|(guid, name, owner, rule)| AbProfile {
            guid,
            name,
            owner,
            rule,
            note: String::new(),
        })
        .collect();

    let total = profiles.len() as i64;
    Ok(Json(AbSharedProfilesResponse {
        data: profiles,
        total,
    }))
}

/// GET /api/ab/settings — address book configuration.
async fn get_ab_settings(AuthUser(_claims): AuthUser) -> Json<Value> {
    Json(json!({
        "max_peer_one_ab": 0
    }))
}
