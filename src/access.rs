use crate::error::ApiError;
use sqlx::SqlitePool;

#[derive(Clone, Copy)]
pub enum Access {
    Read = 1,
    Write = 2,
}

/// The strongest valid direct or group share grants the operation.
pub async fn resolve_book(
    db: &SqlitePool,
    user_id: i64,
    guid: &str,
    access: Access,
) -> Result<String, ApiError> {
    let book: Option<String> = sqlx::query_scalar(
        "SELECT ab.guid FROM address_books ab
         WHERE ((? = '' AND ab.owner_id = ? AND ab.is_personal = TRUE) OR ab.guid = ?)
         AND (ab.owner_id = ? OR EXISTS (
             SELECT 1 FROM ab_shares s WHERE s.ab_guid = ab.guid
             AND s.rule BETWEEN ? AND 3
             AND (s.user_id = ? OR s.group_id IN (
                 SELECT group_id FROM user_groups WHERE user_id = ?))))",
    )
    .bind(guid)
    .bind(user_id)
    .bind(guid)
    .bind(user_id)
    .bind(access as i32)
    .bind(user_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    book.ok_or_else(|| ApiError::Forbidden("Access denied to this address book".into()))
}
