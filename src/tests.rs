use crate::{
    auth::jwt::{create_token, validate_token},
    config::Config,
    state::AppState,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use tower::ServiceExt;

struct Fixture {
    db: SqlitePool,
    app: Router,
    admin: String,
    member: String,
    outsider: String,
}
impl Fixture {
    async fn new() -> Self {
        let db = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        crate::db::run_migrations(&db).await;
        static HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        let hash = HASH.get_or_init(|| {
            crate::auth::password::hash_password("a-secure-test-password").unwrap()
        });
        for (id, name, admin) in [
            (1, "admin", true),
            (2, "member", false),
            (3, "outsider", false),
        ] {
            sqlx::query(
                "INSERT INTO users(id, username, password_hash, is_admin) VALUES (?, ?, ?, ?)",
            )
            .bind(id)
            .bind(name)
            .bind(hash)
            .bind(admin)
            .execute(&db)
            .await
            .unwrap();
        }
        sqlx::raw_sql("INSERT INTO address_books(guid, name, owner_id, is_personal) VALUES ('shared', 'Shared', 1, FALSE), ('personal', 'Personal', 1, TRUE);
            INSERT INTO ab_shares(ab_guid, user_id, rule) VALUES ('shared', 2, 1);
            INSERT INTO tags(ab_guid, name) VALUES ('shared', 'work');
            INSERT INTO peers(ab_guid, rustdesk_id, alias) VALUES ('shared', '100', 'Original');
            INSERT INTO peer_tags(peer_id, tag_id) VALUES (1, 1);")
            .execute(&db).await.unwrap();
        let mut config: Config = toml::from_str("").unwrap();
        config.jwt_secret = "test-secret-longer-than-thirty-two-characters".into();
        let mut tokens = Vec::new();
        for (id, name, admin) in [
            (1, "admin", true),
            (2, "member", false),
            (3, "outsider", false),
        ] {
            let token = create_token(name, id, admin, &config.jwt_secret, 1).unwrap();
            let claims = validate_token(&token, &config.jwt_secret).unwrap();
            sqlx::query("INSERT INTO sessions(id, user_id, expires_at) VALUES (?, ?, ?)")
                .bind(claims.jti)
                .bind(id)
                .bind(claims.exp as i64)
                .execute(&db)
                .await
                .unwrap();
            tokens.push(token);
        }
        let app = Router::new()
            .merge(crate::routes::api_router())
            .fallback(crate::routes::frontend::serve_frontend)
            .with_state(AppState {
                db: db.clone(),
                config,
            });
        Self {
            db,
            app,
            admin: tokens[0].clone(),
            member: tokens[1].clone(),
            outsider: tokens[2].clone(),
        }
    }
    async fn call(
        &self,
        method: &str,
        path: &str,
        token: &str,
        body: Value,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("Content-Type", "application/json");
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        let response = self
            .app
            .clone()
            .oneshot(
                request
                    .body(if body.is_null() {
                        Body::empty()
                    } else {
                        Body::from(body.to_string())
                    })
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 3 * 1024 * 1024)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
}

#[tokio::test]
async fn readonly_share_rejects_all_peer_and_tag_mutations() {
    let f = Fixture::new().await;
    assert_eq!(
        f.call("GET", "/api/ab/peers?ab=shared", &f.member, Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    for (method, path, body) in [
        ("POST", "/api/ab/peer/add/shared", json!({"id":"200"})),
        (
            "PUT",
            "/api/ab/peer/update/shared",
            json!({"id":"100", "alias":"Changed"}),
        ),
        ("DELETE", "/api/ab/peer/shared", json!(["100"])),
        ("POST", "/api/ab/tag/add/shared", json!({"name":"new"})),
        (
            "PUT",
            "/api/ab/tag/rename/shared",
            json!({"old":"work", "new":"renamed"}),
        ),
        (
            "PUT",
            "/api/ab/tag/update/shared",
            json!({"name":"work", "color":1}),
        ),
        ("DELETE", "/api/ab/tag/shared", json!(["work"])),
    ] {
        assert_eq!(
            f.call(method, path, &f.member, body).await.0,
            StatusCode::FORBIDDEN,
            "{path}"
        );
    }
    assert_eq!(
        f.call("GET", "/api/ab/peers?ab=shared", &f.outsider, Value::Null)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn strongest_group_share_grants_write_and_profiles_are_deduplicated() {
    let f = Fixture::new().await;
    sqlx::raw_sql(
        "INSERT INTO groups(id, name) VALUES (1, 'Support'); INSERT INTO user_groups VALUES (2, 1);
        INSERT INTO ab_shares(ab_guid, group_id, rule) VALUES ('shared', 1, 2);",
    )
    .execute(&f.db)
    .await
    .unwrap();
    assert_eq!(
        f.call(
            "POST",
            "/api/ab/peer/add/shared",
            &f.member,
            json!({"id":"200"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, body) = f
        .call("GET", "/api/ab/shared/profiles", &f.member, Value::Null)
        .await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["rule"], 2);
}

#[tokio::test]
async fn logout_revokes_only_that_session() {
    let f = Fixture::new().await;
    assert_eq!(
        f.call("POST", "/api/logout", &f.member, json!({})).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.call("GET", "/api/currentUser", &f.member, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.call("GET", "/api/currentUser", &f.admin, Value::Null)
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn privileges_are_loaded_from_database_not_stale_token() {
    let f = Fixture::new().await;
    sqlx::query("UPDATE users SET is_admin = FALSE WHERE id = 1")
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.call("GET", "/api/users", &f.admin, Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE users SET status = 0 WHERE id = 2")
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.call("GET", "/api/currentUser", &f.member, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn disabled_and_reenabled_user_does_not_regain_old_session() {
    let f = Fixture::new().await;
    for status in [0, 1] {
        assert_eq!(
            f.call("PUT", "/api/users/2", &f.admin, json!({"status":status}))
                .await
                .0,
            StatusCode::OK
        );
    }
    assert_eq!(
        f.call("GET", "/api/currentUser", &f.member, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.call("PUT", "/api/users/1", &f.admin, json!({"is_admin":false}))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn password_change_revokes_sessions() {
    let f = Fixture::new().await;
    assert_eq!(
        f.call(
            "PUT",
            "/api/users/2",
            &f.admin,
            json!({"password":"a-new-secure-password"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        f.call("GET", "/api/currentUser", &f.member, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn partial_update_preserves_other_fields_and_clears_tags() {
    let f = Fixture::new().await;
    assert_eq!(
        f.call(
            "PUT",
            "/api/ab/peer/update/shared",
            &f.admin,
            json!({"id":"100", "alias":"New", "tags":[]})
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, body) = f
        .call("GET", "/api/ab/peers?ab=shared", &f.admin, Value::Null)
        .await;
    assert_eq!(body["data"][0]["alias"], "New");
    assert_eq!(body["data"][0]["tags"], json!([]));
    assert_eq!(body["data"][0]["id"], "100");
    assert_eq!(
        f.call(
            "PUT",
            "/api/ab/peer/update/shared",
            &f.admin,
            json!({"id":"missing", "alias":"New"})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn invalid_tag_rolls_back_peer_update() {
    let f = Fixture::new().await;
    assert_eq!(
        f.call(
            "PUT",
            "/api/ab/peer/update/shared",
            &f.admin,
            json!({"id":"100", "alias":"Lost", "tags":["missing"]})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let (_, body) = f
        .call("GET", "/api/ab/peers?ab=shared", &f.admin, Value::Null)
        .await;
    assert_eq!(body["data"][0]["alias"], "Original");
    assert_eq!(body["data"][0]["tags"], json!(["work"]));
}

#[tokio::test]
async fn pagination_and_filters_cover_records_beyond_first_page() {
    let f = Fixture::new().await;
    for id in 200..305 {
        sqlx::query("INSERT INTO peers(ab_guid, rustdesk_id, alias) VALUES ('shared', ?, ?)")
            .bind(id.to_string())
            .bind(format!("Device {id}"))
            .execute(&f.db)
            .await
            .unwrap();
    }
    let (_, body) = f
        .call(
            "GET",
            "/api/ab/peers?ab=shared&current=2&pageSize=100",
            &f.admin,
            Value::Null,
        )
        .await;
    assert_eq!(body["total"], 106);
    assert_eq!(body["data"].as_array().unwrap().len(), 6);
    let (_, body) = f
        .call(
            "GET",
            "/api/ab/peers?ab=shared&q=DEVICE%20304",
            &f.admin,
            Value::Null,
        )
        .await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["id"], "304");
    let (_, body) = f
        .call(
            "GET",
            "/api/ab/peers?ab=shared&tag=work",
            &f.admin,
            Value::Null,
        )
        .await;
    assert_eq!(body["total"], 1);
    for query in [
        "pageSize=-1",
        "pageSize=101",
        "current=0",
        "current=9223372036854775807&pageSize=100",
    ] {
        assert_eq!(
            f.call(
                "GET",
                &format!("/api/ab/peers?ab=shared&{query}"),
                &f.admin,
                Value::Null
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn legacy_replacement_rolls_back_on_duplicate_ids() {
    let f = Fixture::new().await;
    sqlx::query("INSERT INTO peers(ab_guid, rustdesk_id) VALUES ('personal', 'original')")
        .execute(&f.db)
        .await
        .unwrap();
    let data =
        json!({"peers":[{"id":"duplicate"},{"id":"duplicate"}],"tags":["new"],"tag_colors":"{}"});
    assert_eq!(
        f.call(
            "POST",
            "/api/ab",
            &f.admin,
            json!({"data":data.to_string()})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM peers WHERE rustdesk_id = 'original'")
            .fetch_one(&f.db)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn deletes_accept_client_lists_and_web_objects_and_cascade() {
    let f = Fixture::new().await;
    assert_eq!(
        f.call("DELETE", "/api/ab/tag/shared", &f.admin, json!(["work"]))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        f.call(
            "DELETE",
            "/api/ab/peer/shared",
            &f.admin,
            json!({"ids":["100"]})
        )
        .await
        .0,
        StatusCode::OK
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM peer_tags")
        .fetch_one(&f.db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn audit_requires_admin_and_events_require_identity_by_default() {
    let f = Fixture::new().await;
    assert_eq!(
        f.call("GET", "/api/audit-log", &f.member, Value::Null)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        f.call("POST", "/api/audit", "", json!({"action":"connect"}))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.call("POST", "/api/audit", &f.member, json!({"action":"connect"}))
            .await
            .0,
        StatusCode::OK
    );
    let (_, body) = f
        .call("GET", "/api/audit-log?pageSize=1", &f.admin, Value::Null)
        .await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["data"][0]["user_id"], 2);
    assert_eq!(
        f.call("GET", "/api/unknown", "", Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn migrations_are_atomic_idempotent_and_detect_edits() {
    let f = Fixture::new().await;
    crate::db::run_migrations(&f.db).await;
    let bad = [(
        "009_bad.sql",
        "CREATE TABLE rollback_test(id INTEGER); INSERT INTO missing_table VALUES(1);",
    )];
    assert!(crate::db::apply_migrations(&f.db, &bad).await.is_err());
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'rollback_test')",
    )
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert!(!exists);
    let good = [(
        "009_good.sql",
        "CREATE TABLE literal_test(value TEXT); INSERT INTO literal_test VALUES ('a;b');",
    )];
    crate::db::apply_migrations(&f.db, &good).await.unwrap();
    let value: String = sqlx::query_scalar("SELECT value FROM literal_test")
        .fetch_one(&f.db)
        .await
        .unwrap();
    assert_eq!(value, "a;b");
    assert!(
        crate::db::apply_migrations(&f.db, &[("009_good.sql", "SELECT 1;")])
            .await
            .is_err()
    );
}

#[tokio::test]
async fn personal_book_is_unique_and_repeated_reads_are_idempotent() {
    let f = Fixture::new().await;
    for _ in 0..3 {
        assert_eq!(
            f.call("GET", "/api/ab/personal", &f.member, Value::Null)
                .await
                .0,
            StatusCode::OK
        );
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM address_books WHERE owner_id = 2 AND is_personal = TRUE",
    )
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn login_issues_distinct_persistent_sessions_and_rejects_bad_password() {
    let f = Fixture::new().await;
    let (status, first) = f
        .call(
            "POST",
            "/api/login",
            "",
            json!({"username":"member", "password":"a-secure-test-password"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["type"], "access_token");
    let token = first["access_token"].as_str().unwrap();
    assert_ne!(token, f.member);
    assert_eq!(
        f.call("GET", "/api/currentUser", token, Value::Null)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        f.call(
            "POST",
            "/api/login",
            "",
            json!({"username":"member", "password":"wrong"})
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn config_rejects_unsafe_secrets_origins_and_expiry() {
    let mut config: Config = toml::from_str("").unwrap();
    assert!(config.validate().is_err());
    config.jwt_secret = "a-valid-test-secret-with-at-least-32-characters".into();
    assert!(config.validate().is_ok());
    config.cors_origins = vec!["*".into()];
    assert!(config.validate().is_err());
    config.cors_origins = vec!["https://example.com".into()];
    assert!(config.validate().is_ok());
    config.token_expiry_hours = u64::MAX;
    assert!(config.validate().is_err());
    config.token_expiry_hours = 1;
    config.jwt_secret = "weak".into();
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn admin_routes_validate_groups_and_reject_non_admins() {
    let f = Fixture::new().await;
    for path in ["/api/users", "/api/groups"] {
        assert_eq!(
            f.call("GET", path, &f.member, Value::Null).await.0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        f.call("POST", "/api/groups", &f.admin, json!({"name":"   "}))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.call("POST", "/api/groups", &f.admin, json!({"name":"Support"}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        f.call("POST", "/api/groups", &f.admin, json!({"name":"Support"}))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        f.call(
            "PUT",
            "/api/groups/99999",
            &f.admin,
            json!({"name":"Missing"})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        f.call("PUT", "/api/users/1", &f.admin, json!({"is_admin":false}))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn expired_or_unregistered_sessions_are_rejected_even_with_valid_signature() {
    let f = Fixture::new().await;
    sqlx::query("UPDATE sessions SET expires_at = unixepoch() - 1 WHERE user_id = 2")
        .execute(&f.db)
        .await
        .unwrap();
    assert_eq!(
        f.call("GET", "/api/currentUser", &f.member, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let token = create_token(
        "admin",
        1,
        true,
        "test-secret-longer-than-thirty-two-characters",
        1,
    )
    .unwrap();
    assert_eq!(
        f.call("GET", "/api/currentUser", &token, Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn login_rate_limit_also_covers_unknown_users() {
    let f = Fixture::new().await;
    let username = format!("missing-{}", uuid::Uuid::new_v4());
    for _ in 0..10 {
        assert_eq!(
            f.call(
                "POST",
                "/api/login",
                "",
                json!({"username":username,"password":"wrong"})
            )
            .await
            .0,
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        f.call(
            "POST",
            "/api/login",
            "",
            json!({"username":username,"password":"wrong"})
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn client_bodyless_posts_preserve_get_contracts_and_enforce_authentication() {
    let f = Fixture::new().await;
    for path in [
        "/api/ab/personal",
        "/api/ab/settings",
        "/api/ab/shared/profiles",
        "/api/ab/peers?ab=shared",
        "/api/ab/tags/shared",
    ] {
        assert_eq!(
            f.call("POST", path, "", Value::Null).await.0,
            StatusCode::UNAUTHORIZED,
            "{path}"
        );
        assert_eq!(
            f.call("POST", path, &f.member, Value::Null).await.0,
            StatusCode::OK,
            "{path}"
        );
    }
    let (_, post) = f
        .call("POST", "/api/ab/personal", &f.admin, Value::Null)
        .await;
    let (_, get) = f
        .call("GET", "/api/ab/personal", &f.admin, Value::Null)
        .await;
    assert_eq!(post["guid"], "personal");
    assert!(post.get("data").is_none());
    assert_eq!(get["data"], post);
    let (_, post) = f
        .call("POST", "/api/ab/tags/shared", &f.member, Value::Null)
        .await;
    let (_, get) = f
        .call("GET", "/api/ab/tags/shared", &f.member, Value::Null)
        .await;
    assert!(post.is_array());
    assert_eq!(post[0]["name"], "work");
    assert_eq!(get["data"], post);
    let (_, get) = f
        .call("GET", "/api/ab/settings", &f.member, Value::Null)
        .await;
    let (_, post) = f
        .call("POST", "/api/ab/settings", &f.member, Value::Null)
        .await;
    assert_eq!(get, post);
    assert_eq!(post["max_peer_one_ab"], 0);
    for path in [
        "/api/ab/shared/profiles",
        "/api/ab/peers?ab=shared&current=1&pageSize=1",
    ] {
        let (_, get) = f.call("GET", path, &f.member, Value::Null).await;
        let (_, post) = f.call("POST", path, &f.member, Value::Null).await;
        assert_eq!(get, post);
        assert!(post["data"].is_array());
    }
    assert_eq!(
        f.call(
            "POST",
            "/api/ab/peers?ab=shared&pageSize=-1",
            &f.member,
            Value::Null
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    for path in ["/api/ab/tags/shared", "/api/ab/peers?ab=shared"] {
        assert_eq!(
            f.call("POST", path, &f.outsider, Value::Null).await.0,
            StatusCode::FORBIDDEN
        );
    }
    let request = Request::builder()
        .method("POST")
        .uri("/api/ab/personal")
        .header("Authorization", format!("Bearer {}", f.admin))
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        f.app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn device_inventory_is_paginated_and_never_exposed_to_ordinary_accounts() {
    let f = Fixture::new().await;
    sqlx::raw_sql("INSERT INTO devices(rustdesk_id, hostname, platform, version) VALUES ('001234', 'Office', 'Windows', '1.test');
        INSERT INTO devices(rustdesk_id, hostname, platform, os, last_online) VALUES
          ('002345', 'Offline', 'Linux', 'Linux / Debian', datetime('now', '-1 day')),
          ('003456', 'Future', 'Linux', '', datetime('now', '+1 day'));")
        .execute(&f.db).await.unwrap();
    let path = "/api/peers?accessible=&status=1&current=1&pageSize=1";
    let (status, first) = f.call("GET", path, &f.admin, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["total"], 3);
    assert_eq!(first["data"].as_array().unwrap().len(), 1);
    let peer = &first["data"][0];
    assert_eq!(peer["id"], "001234");
    assert!(peer["id"].is_string());
    assert_eq!(
        peer["info"],
        json!({"device_name":"Office", "os":"Windows", "version":"1.test", "username":""})
    );
    assert_eq!(peer["status"], 1);
    for key in ["user", "user_name", "device_group_name", "note"] {
        assert_eq!(peer[key], "");
    }
    let (_, second) = f
        .call(
            "GET",
            "/api/peers?accessible=&status=1&current=2&pageSize=1",
            &f.admin,
            Value::Null,
        )
        .await;
    assert_eq!(second["data"][0]["id"], "002345");
    assert_eq!(second["data"][0]["status"], 0);
    assert_eq!(second["data"][0]["info"]["os"], "Linux / Debian");
    let (_, third) = f
        .call(
            "GET",
            "/api/peers?current=3&pageSize=1",
            &f.admin,
            Value::Null,
        )
        .await;
    assert_eq!(third["data"][0]["status"], 0);
    for token in [&f.member, &f.outsider] {
        let (status, response) = f.call("GET", path, token, Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response, json!({"data":[],"total":0}));
    }
    assert_eq!(
        f.call("GET", path, "", Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    for query in [
        "pageSize=-1",
        "pageSize=101",
        "current=0",
        "current=9223372036854775807&pageSize=100",
    ] {
        assert_eq!(
            f.call("GET", &format!("/api/peers?{query}"), &f.admin, Value::Null)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[tokio::test]
async fn accessible_device_groups_do_not_reuse_user_groups() {
    let f = Fixture::new().await;
    sqlx::query("INSERT INTO groups(name) VALUES ('User group, not a device group')")
        .execute(&f.db)
        .await
        .unwrap();
    for token in [&f.admin, &f.member] {
        let (status, body) = f
            .call(
                "GET",
                "/api/device-group/accessible?current=1&pageSize=100",
                token,
                Value::Null,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, json!({"data":[],"total":0}));
    }
    assert_eq!(
        f.call("GET", "/api/device-group/accessible", "", Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.call(
            "GET",
            "/api/device-group/accessible?pageSize=101",
            &f.admin,
            Value::Null
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn accessible_users_use_login_names_and_hide_inactive_or_other_users() {
    let f = Fixture::new().await;
    sqlx::raw_sql(
        "UPDATE users SET name = 'Member Display' WHERE id = 2;
        UPDATE users SET status = 0 WHERE id = 3;",
    )
    .execute(&f.db)
    .await
    .unwrap();
    let path = "/api/users?accessible=&status=1&current=1&pageSize=100";
    let (status, members) = f.call("GET", path, &f.member, Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(members["total"], 1);
    assert_eq!(
        members["data"][0],
        json!({"name":"member", "display_name":"Member Display", "status":1})
    );
    let (_, admins) = f.call("GET", path, &f.admin, Value::Null).await;
    assert_eq!(admins["total"], 2);
    assert_eq!(admins["data"].as_array().unwrap().len(), 2);
    let (_, page) = f
        .call(
            "GET",
            "/api/users?accessible=&status=1&current=2&pageSize=1",
            &f.admin,
            Value::Null,
        )
        .await;
    assert_eq!(page["total"], 2);
    assert_eq!(page["data"], members["data"]);
    let (_, empty) = f
        .call(
            "GET",
            "/api/users?accessible=&status=0",
            &f.admin,
            Value::Null,
        )
        .await;
    assert_eq!(empty, json!({"data":[],"total":0}));
    assert_eq!(
        f.call("GET", path, "", Value::Null).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        f.call(
            "GET",
            "/api/users?accessible=&pageSize=101",
            &f.admin,
            Value::Null
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.call(
            "GET",
            "/api/users?accessible=&status=2",
            &f.admin,
            Value::Null
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        f.call("GET", "/api/users", &f.member, Value::Null).await.0,
        StatusCode::FORBIDDEN
    );
    let (_, web) = f.call("GET", "/api/users", &f.admin, Value::Null).await;
    assert_eq!(web["total"], 3);
    assert_eq!(web["data"][1]["username"], "member");
    assert_eq!(web["data"][1]["name"], "Member Display");
    assert!(web["data"][1].get("password_hash").is_none());
}

#[tokio::test]
async fn running_server_version_is_public_and_not_cacheable() {
    let f = Fixture::new().await;
    let response = f
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/version")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body,
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "commit": option_env!("RUSTBOOK_BUILD_COMMIT").unwrap_or("unknown"),
        })
    );
}
