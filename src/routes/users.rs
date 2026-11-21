use axum::{
    extract::{Path, Query, State},
    routing::{get, put},
    Json, Router,
};
use serde_json::{json, Value};

use crate::auth::middleware::AuthUser;
use crate::auth::password::hash_async;
use crate::error::ApiError;
use crate::models::user::*;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(list_users).post(create_user))
        .route("/api/users/{id}", put(update_user).delete(delete_user))
}

fn require_admin(claims: &crate::auth::jwt::Claims) -> Result<(), ApiError> {
    if !claims.is_admin {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct UsersQuery {
    accessible: Option<String>,
    status: Option<i32>,
    #[serde(default = "crate::pagination::default_page")]
    current: i64,
    #[serde(default = "crate::pagination::default_size", rename = "pageSize")]
    page_size: i64,
}

#[derive(sqlx::FromRow, serde::Serialize)]
struct AccessibleUser {
    name: String,
    display_name: String,
    status: i32,
}

async fn list_users(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(query): Query<UsersQuery>,
) -> Result<Json<Value>, ApiError> {
    if query.accessible.is_some() {
        let offset = crate::pagination::Pagination {
            current: query.current,
            page_size: query.page_size,
        }
        .offset()?;
        if query
            .status
            .is_some_and(|status| status != 0 && status != 1)
        {
            return Err(ApiError::BadRequest("status must be 0 or 1".into()));
        }
        // Accessible profiles never disclose inactive users or admin-only fields.
        let requested_status = query.status.unwrap_or(1);
        let data: Vec<AccessibleUser> = sqlx::query_as(
            "SELECT username AS name, name AS display_name, status FROM users
             WHERE status = 1 AND ? = 1 AND (? OR id = ?) ORDER BY id LIMIT ? OFFSET ?",
        )
        .bind(requested_status)
        .bind(claims.is_admin)
        .bind(claims.user_id)
        .bind(query.page_size)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM users WHERE status = 1 AND ? = 1 AND (? OR id = ?)",
        )
        .bind(requested_status)
        .bind(claims.is_admin)
        .bind(claims.user_id)
        .fetch_one(&state.db)
        .await?;
        return Ok(Json(json!({"data": data, "total": total})));
    }
    require_admin(&claims)?;

    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY id")
        .fetch_all(&state.db)
        .await?;

    let items: Vec<UserListItem> = users
        .into_iter()
        .map(|u| UserListItem {
            id: u.id,
            username: u.username,
            name: u.name,
            email: u.email,
            is_admin: u.is_admin,
            status: u.status,
            created_at: u.created_at,
        })
        .collect();

    let total = items.len();
    Ok(Json(json!({ "data": items, "total": total })))
}

async fn create_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&claims)?;

    if req.username.trim().is_empty() || req.username.len() > 128 {
        return Err(ApiError::BadRequest("Invalid username".into()));
    }
    validate_password(&req.password)?;
    let password_hash = hash_async(req.password).await?;

    sqlx::query(
        "INSERT INTO users (username, password_hash, name, email, is_admin) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&req.username)
    .bind(&password_hash)
    .bind(&req.name)
    .bind(&req.email)
    .bind(req.is_admin)
    .execute(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
            ApiError::Conflict(format!("Username '{}' already exists", req.username))
        }
        _ => ApiError::Internal(e.to_string()),
    })?;

    Ok(Json(json!({})))
}

async fn update_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&claims)?;

    if req.status.is_some_and(|status| status != 0 && status != 1) {
        return Err(ApiError::BadRequest("status must be 0 or 1".into()));
    }
    if id == claims.user_id && (req.is_admin == Some(false) || req.status == Some(0)) {
        return Err(ApiError::BadRequest(
            "Cannot disable or demote your own account".into(),
        ));
    }
    let revoke = req.password.is_some() || req.status.is_some();
    let password = if let Some(password) = req.password {
        validate_password(&password)?;
        Some(hash_async(password).await?)
    } else {
        None
    };
    let mut tx = state.db.begin().await?;
    let result = sqlx::query("UPDATE users SET name = COALESCE(?, name), email = COALESCE(?, email),
        is_admin = COALESCE(?, is_admin), status = COALESCE(?, status), password_hash = COALESCE(?, password_hash),
        updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(req.name).bind(req.email).bind(req.is_admin).bind(req.status).bind(password).bind(id)
        .execute(&mut *tx).await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("User not found".into()));
    }
    if revoke {
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    Ok(Json(json!({})))
}

async fn delete_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&claims)?;

    if id == claims.user_id {
        return Err(ApiError::BadRequest("Cannot delete yourself".to_string()));
    }

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({})))
}

fn validate_password(password: &str) -> Result<(), ApiError> {
    if !(12..=1024).contains(&password.len()) {
        return Err(ApiError::BadRequest(
            "Password must contain 12–1024 bytes".into(),
        ));
    }
    Ok(())
}
