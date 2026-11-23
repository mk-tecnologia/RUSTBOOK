use crate::{auth::middleware::AuthUser, error::ApiError, state::AppState};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, net::IpAddr, path::Path, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    time::timeout,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/server-settings", get(read).put(save))
        .route("/api/admin/server-status", get(health))
        .route("/api/admin/server-settings/check", post(check))
        .route("/api/admin/server-settings/apply-relay", post(apply))
        .route("/api/client-configuration", get(clients))
}
#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub id_server: String,
    #[serde(default)]
    pub relay_server: String,
    #[serde(default)]
    pub api_url: String,
    #[serde(default)]
    pub public_key: String,
    #[serde(default)]
    pub presence_server: String,
    #[serde(default)]
    pub allow_private_relay: bool,
    #[serde(default)]
    pub revision: String,
}
fn admin(claims: &crate::auth::jwt::Claims) -> Result<(), ApiError> {
    if claims.is_admin {
        Ok(())
    } else {
        Err(ApiError::Forbidden("Admin access required".into()))
    }
}
async fn load(state: &AppState) -> Result<Settings, ApiError> {
    let rows: Vec<(String,String)> = sqlx::query_as("SELECT key,value FROM server_settings WHERE key IN ('native_connection','rendezvous_server','server_network','config_revision')")
        .fetch_all(&state.db).await?;
    let rows: HashMap<_, _> = rows.into_iter().collect();
    let mut settings: Settings = rows
        .get("server_network")
        .map(|s| serde_json::from_str(s))
        .transpose()
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .unwrap_or_default();
    if let Some(native) = rows.get("native_connection") {
        let value: Value =
            serde_json::from_str(native).map_err(|e| ApiError::Internal(e.to_string()))?;
        settings.id_server = value["server"].as_str().unwrap_or_default().into();
        settings.public_key = value["public_key"].as_str().unwrap_or_default().into();
    }
    settings.presence_server = rows
        .get("rendezvous_server")
        .cloned()
        .unwrap_or_else(|| state.config.rendezvous_server.clone());
    settings.revision = rows.get("config_revision").cloned().unwrap_or_default();
    Ok(settings)
}
pub fn validate_endpoint(value: &str) -> Result<(), ApiError> {
    let authority = value.parse::<axum::http::uri::Authority>().map_err(|_| {
        ApiError::BadRequest("Use hostname:port without protocol, path or credentials".into())
    })?;
    if value.len() > 255
        || authority.host().is_empty()
        || authority.port_u16().is_none_or(|port| port == 0)
        || value
            .chars()
            .any(|c| c.is_whitespace() || "@/\\?#$\"'".contains(c))
    {
        return Err(ApiError::BadRequest(
            "Use hostname:port without protocol, path or credentials".into(),
        ));
    }
    let host = authority.host().trim_matches(['[', ']']);
    if host.parse::<IpAddr>().is_err()
        && (host.contains(':')
            || !host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
            || !host
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric()))
    {
        return Err(ApiError::BadRequest(
            "Invalid endpoint hostname or IP address".into(),
        ));
    }
    Ok(())
}
fn private_endpoint(value: &str) -> bool {
    let Ok(authority) = value.parse::<axum::http::uri::Authority>() else {
        return false;
    };
    let host = authority.host().trim_matches(['[', ']']);
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            ip.is_private() || ip.is_loopback() || ip.is_link_local() || ip.is_unspecified()
        }
        Ok(IpAddr::V6(ip)) => {
            ip.is_loopback()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
                || ip.is_unspecified()
        }
        Err(_) => {
            !host.contains('.')
                || host.ends_with(".local")
                || host.ends_with(".internal")
                || host.ends_with(".lan")
        }
    }
}
fn warnings(settings: &Settings) -> Vec<&'static str> {
    let mut result = Vec::new();
    if !settings.relay_server.is_empty() && private_endpoint(&settings.relay_server) {
        result.push("O relay anunciado é privado/local. Clientes externos precisam de domínio/IP público alcançável ou VPN.");
    }
    if !settings.id_server.is_empty() && private_endpoint(&settings.id_server) {
        result
            .push("O endereço do servidor de ID é privado/local. Confira o acesso a partir de todos os clientes.");
    }
    if settings.api_url.starts_with("http://") {
        result.push("A API usa HTTP. Use HTTPS para credenciais transmitidas pela Internet.");
    }
    if settings.relay_server.is_empty() {
        result.push("O endereço público do relay ainda não foi configurado.");
    }
    result
}
fn validate(settings: &Settings) -> Result<(), ApiError> {
    for endpoint in [
        &settings.id_server,
        &settings.relay_server,
        &settings.presence_server,
    ] {
        if !endpoint.is_empty() {
            validate_endpoint(endpoint)?;
        }
    }
    if (!settings.id_server.is_empty() || !settings.public_key.is_empty())
        && (settings.id_server.is_empty()
            || settings.public_key.len() != 44
            || STANDARD
                .decode(&settings.public_key)
                .map_or(true, |key| key.len() != 32))
    {
        return Err(ApiError::BadRequest("Provide the ID server and its existing 32-byte Base64 public Key together. Never use the private key.".into()));
    }
    if !settings.api_url.is_empty() {
        if settings
            .api_url
            .chars()
            .any(|c| c.is_whitespace() || "@#\\".contains(c))
        {
            return Err(ApiError::BadRequest(
                "Use an HTTP(S) API origin without credentials or fragments".into(),
            ));
        }
        let uri = settings.api_url.parse::<axum::http::Uri>().map_err(|_| {
            ApiError::BadRequest("Use an HTTP(S) API origin without path or credentials".into())
        })?;
        if !matches!(uri.scheme_str(), Some("http" | "https"))
            || uri.authority().is_none()
            || uri.path() != "/"
            || uri.query().is_some()
        {
            return Err(ApiError::BadRequest(
                "Use an HTTP(S) API origin without path or credentials".into(),
            ));
        }
        let host = uri.host().unwrap_or_default();
        validate_endpoint(&format!(
            "{}:{}",
            host,
            uri.port_u16()
                .unwrap_or(if uri.scheme_str() == Some("https") {
                    443
                } else {
                    80
                })
        ))?;
    }
    Ok(())
}
pub async fn bump_revision(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<String, sqlx::Error> {
    let revision = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO server_settings(key,value) VALUES ('config_revision',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value")
        .bind(&revision).execute(&mut **tx).await?;
    Ok(revision)
}
async fn agent(state: &AppState, request: Value, seconds: u64) -> Result<Value, String> {
    #[cfg(unix)]
    {
        let path = Path::new(&state.config.db_path)
            .parent()
            .unwrap_or(Path::new("."))
            .join(".rustbook-server-agent.sock");
        timeout(Duration::from_secs(seconds), async {
            let mut stream = tokio::net::UnixStream::connect(&path)
                .await
                .map_err(|e| e.to_string())?;
            let mut body = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
            body.push(b'\n');
            stream.write_all(&body).await.map_err(|e| e.to_string())?;
            let mut reader = BufReader::new(stream).take(16385);
            let mut response = String::new();
            reader
                .read_line(&mut response)
                .await
                .map_err(|e| e.to_string())?;
            if response.len() > 16384 {
                return Err("Invalid agent response".into());
            }
            serde_json::from_str(&response).map_err(|e| e.to_string())
        })
        .await
        .map_err(|_| "Server agent did not respond in time".to_string())?
    }
    #[cfg(not(unix))]
    {
        let _ = (state, request, seconds);
        Err("Server agent requires a Linux host".into())
    }
}
async fn validate_managed_key(state: &AppState, settings: &Settings) -> Result<(), ApiError> {
    if settings.public_key.is_empty() {
        return Ok(());
    }
    let status = match agent(state, json!({"action":"status"}), 5).await {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };
    let existing = status["public_key"].as_str().unwrap_or_default();
    if !existing.is_empty() && existing != settings.public_key {
        return Err(ApiError::BadRequest("The public Key differs from the managed hbbs key. Use the existing server key; do not regenerate it.".into()));
    }
    Ok(())
}
async fn report(state: &AppState, settings: Settings) -> Value {
    let status = agent(state, json!({"action":"status"}), 5)
        .await
        .unwrap_or_else(|_| json!({"connected":false}));
    let mut messages = warnings(&settings);
    if !settings.public_key.is_empty()
        && status["public_key"]
            .as_str()
            .is_some_and(|key| !key.is_empty() && key != settings.public_key)
    {
        messages.push("A Key salva difere da chave pública existente no hbbs gerenciado. Use a chave existente antes de configurar os clientes.");
    }
    json!({"settings":settings,"warnings":messages,"agent":status,
        "runtime":{"port":state.config.port,"token_expiry_hours":state.config.token_expiry_hours,"cors_origins":state.config.cors_origins,"anonymous_device_events":state.config.allow_unauthenticated_device_events}})
}
async fn read(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    admin(&claims)?;
    Ok(Json(report(&state, load(&state).await?).await))
}
async fn save(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(mut settings): Json<Settings>,
) -> Result<Json<Value>, ApiError> {
    admin(&claims)?;
    settings.id_server = settings.id_server.trim().into();
    settings.relay_server = settings.relay_server.trim().into();
    settings.api_url = settings.api_url.trim().trim_end_matches('/').into();
    settings.public_key = settings.public_key.trim().into();
    settings.presence_server = settings.presence_server.trim().into();
    validate(&settings)?;
    validate_managed_key(&state, &settings).await?;
    let mut tx = state.db.begin_with("BEGIN IMMEDIATE").await?;
    let current: Option<String> =
        sqlx::query_scalar("SELECT value FROM server_settings WHERE key='config_revision'")
            .fetch_optional(&mut *tx)
            .await?;
    if current.unwrap_or_default() != settings.revision {
        return Err(ApiError::Conflict(
            "Settings changed. Reload before saving.".into(),
        ));
    }
    let native = json!({"server":settings.id_server,"public_key":settings.public_key}).to_string();
    for (key, value) in [
        ("native_connection", native),
        ("rendezvous_server", settings.presence_server.clone()),
        (
            "server_network",
            serde_json::to_string(&settings).map_err(|e| ApiError::Internal(e.to_string()))?,
        ),
    ] {
        sqlx::query("INSERT INTO server_settings(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(key).bind(value).execute(&mut *tx).await?;
    }
    settings.revision = bump_revision(&mut tx).await?;
    sqlx::query(
        "INSERT INTO audit_log(user_id, action, note) VALUES (?, 'server_settings_saved', ?)",
    )
    .bind(claims.user_id)
    .bind(json!({"revision":settings.revision,"relay":settings.relay_server}).to_string())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(report(&state, settings).await))
}
async fn clients(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    let settings = load(&state).await?;
    Ok(Json(
        json!({"id_server":settings.id_server,"relay_server":settings.relay_server,"api_url":settings.api_url,"public_key":settings.public_key,"can_edit":claims.is_admin}),
    ))
}
async fn tcp(endpoint: &str) -> &'static str {
    if endpoint.is_empty() {
        return "not_configured";
    }
    match timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(endpoint),
    )
    .await
    {
        Ok(Ok(_)) => "tcp_reachable",
        _ => "unreachable",
    }
}
async fn presence_check(endpoint: &str) -> &'static str {
    if endpoint.is_empty() {
        "not_configured"
    } else if crate::presence::query(endpoint, &["rustbook-presence-check".into()])
        .await
        .is_ok()
    {
        "protocol_replied"
    } else {
        "unreachable"
    }
}
async fn diagnostics(settings: &Settings) -> Value {
    let api_endpoint = settings
        .api_url
        .parse::<axum::http::Uri>()
        .ok()
        .and_then(|uri| {
            Some(format!(
                "{}:{}",
                uri.host()?,
                uri.port_u16()
                    .unwrap_or(if uri.scheme_str() == Some("https") {
                        443
                    } else {
                        80
                    })
            ))
        })
        .unwrap_or_default();
    let (id, relay, presence, api) = tokio::join!(
        tcp(&settings.id_server),
        tcp(&settings.relay_server),
        presence_check(&settings.presence_server),
        tcp(&api_endpoint)
    );
    json!({"id_server":id,"relay_server":relay,"presence_server":presence,"api_url":api,
        "revision":settings.revision,"notice":"TCP reachability does not validate relay authentication, TLS or a connection between external clients."})
}
async fn check(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    admin(&claims)?;
    Ok(Json(diagnostics(&load(&state).await?).await))
}
async fn health(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> Result<Json<Value>, ApiError> {
    admin(&claims)?;
    let settings = load(&state).await?;
    let (report, checks) = tokio::join!(report(&state, settings.clone()), diagnostics(&settings));
    Ok(Json(json!({"report":report,"checks":checks})))
}
#[derive(Deserialize)]
struct Apply {
    revision: String,
}
async fn apply(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(request): Json<Apply>,
) -> Result<Json<Value>, ApiError> {
    admin(&claims)?;
    let settings = load(&state).await?;
    if request.revision != settings.revision {
        return Err(ApiError::Conflict(
            "Settings changed. Reload before applying.".into(),
        ));
    }
    validate_endpoint(&settings.relay_server)?;
    validate_managed_key(&state, &settings).await?;
    if private_endpoint(&settings.relay_server) && !settings.allow_private_relay {
        return Err(ApiError::BadRequest(
            "Private relay address: confirm internal/VPN-only use or choose a public endpoint."
                .into(),
        ));
    }
    sqlx::query(
        "INSERT INTO audit_log(user_id, action, note) VALUES (?, 'hbbs_relay_apply_requested', ?)",
    )
    .bind(claims.user_id)
    .bind(json!({"revision":settings.revision,"relay":settings.relay_server}).to_string())
    .execute(&state.db)
    .await?;
    let result = agent(
        &state,
        json!({"action":"apply_relay","relay":settings.relay_server}),
        120,
    )
    .await
    .map_err(|_| {
        ApiError::BadRequest(
            "No confirmation from the server agent. Reload the current hbbs status before retrying. If not installed, enable the agent on the host."
                .into(),
        )
    })?;
    if result["ok"] != true {
        return Err(ApiError::BadRequest(
            result["error"]
                .as_str()
                .unwrap_or("Agent could not apply relay")
                .into(),
        ));
    }
    if result["keys_preserved"] != true || result["relay"] != settings.relay_server {
        return Err(ApiError::BadRequest(
            "Incomplete agent confirmation. Reload server status before retrying.".into(),
        ));
    }
    if let Err(error) = sqlx::query("INSERT INTO audit_log(user_id, action, note) VALUES (?, 'hbbs_relay_applied', ?)")
        .bind(claims.user_id).bind(json!({"revision":settings.revision,"relay":settings.relay_server,"changed":result["changed"]}).to_string()).execute(&state.db).await {
        tracing::warn!("Relay applied, but completion audit could not be written: {error}");
    }
    Ok(Json(result))
}
