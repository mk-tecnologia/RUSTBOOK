#![allow(dead_code)]

mod access;
mod auth;
mod config;
mod db;
mod error;
mod models;
mod pagination;
mod routes;
mod state;

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::auth::password::hash_password;
use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn")),
        )
        .init();

    let config = Config::load();
    tracing::info!(
        "Starting RustDesk Address Book Server on port {}",
        config.port
    );

    // Initialize database
    let pool = db::init_pool(&config.db_path).await;
    db::run_migrations(&pool).await;

    // First-run: create admin user if no users exist
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    if user_count == 0 {
        assert!(
            (12..=1024).contains(&config.admin_password.len()) && config.admin_password != "admin",
            "Set RUSTDESK_AB_ADMIN_PASSWORD to at least 12 characters for first-run setup"
        );
        let password_hash =
            hash_password(&config.admin_password).expect("Failed to hash admin password");

        sqlx::query(
            "INSERT INTO users (username, password_hash, name, is_admin) VALUES (?, ?, 'Administrator', TRUE)",
        )
        .bind(&config.admin_username)
        .bind(&password_hash)
        .execute(&pool)
        .await
        .expect("Failed to create admin user");

        tracing::info!(
            "Created default admin user: {} (change password immediately!)",
            config.admin_username
        );
    }

    let state = AppState {
        db: pool,
        config: config.clone(),
    };

    // Same-origin by default; explicit opt-in for cross-origin web clients.
    let cors = CorsLayer::new()
        .allow_origin(
            config
                .cors_origins
                .iter()
                .map(|origin| {
                    origin
                        .parse::<axum::http::HeaderValue>()
                        .expect("Invalid CORS origin")
                })
                .collect::<Vec<_>>(),
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);

    let app = Router::new()
        .merge(routes::api_router())
        .fallback(routes::frontend::serve_frontend)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    tracing::info!("Listening on http://{}", addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown())
        .await
        .expect("Server error");
}

async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}

#[cfg(test)]
mod tests;
