pub mod db;
pub mod tailscale;

use std::{env, path::Path};

use anyhow::{Context, Result};
use axum::{
    extract::{Json, Path as AxumPath, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use tokio::signal;

use crate::control::{
    discovery, LocalAuthRequest, LocalAuthResponse, MapRequest, MapResponse, RegisterRequest,
    RegisterResponse, TailnetDiscovery, BURROW_TAILNET_DOMAIN,
};

#[derive(Clone, Debug)]
pub struct BootstrapIdentity {
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub password_file: String,
}

impl Default for BootstrapIdentity {
    fn default() -> Self {
        Self {
            username: "contact".to_owned(),
            email: "contact@burrow.net".to_owned(),
            display_name: "Burrow Contact".to_owned(),
            password_file: "intake/forgejo_pass_contact_at_burrow_net.txt".to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AuthServerConfig {
    pub listen: String,
    pub db_path: String,
    pub tailnet_domain: String,
    pub bootstrap: BootstrapIdentity,
}

impl Default for AuthServerConfig {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:8080".to_owned(),
            db_path: db::PATH.to_owned(),
            tailnet_domain: BURROW_TAILNET_DOMAIN.to_owned(),
            bootstrap: BootstrapIdentity::default(),
        }
    }
}

impl AuthServerConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(value) = env::var("BURROW_AUTH_LISTEN") {
            config.listen = value;
        }
        if let Ok(value) = env::var("BURROW_AUTH_DB_PATH") {
            config.db_path = value;
        }
        if let Ok(value) = env::var("BURROW_AUTH_TAILNET_DOMAIN") {
            config.tailnet_domain = value;
        }
        if let Ok(value) = env::var("BURROW_BOOTSTRAP_USERNAME") {
            config.bootstrap.username = value;
        }
        if let Ok(value) = env::var("BURROW_BOOTSTRAP_EMAIL") {
            config.bootstrap.email = value;
        }
        if let Ok(value) = env::var("BURROW_BOOTSTRAP_DISPLAY_NAME") {
            config.bootstrap.display_name = value;
        }
        if let Ok(value) = env::var("BURROW_BOOTSTRAP_PASSWORD_FILE") {
            config.bootstrap.password_file = value;
        }
        config
    }

    fn bootstrap_password(&self) -> Result<Option<String>> {
        let path = Path::new(&self.bootstrap.password_file);
        if !path.exists() {
            return Ok(None);
        }
        let password = std::fs::read_to_string(path).with_context(|| {
            format!("failed to read bootstrap password from {}", path.display())
        })?;
        let password = password.trim().to_owned();
        if password.is_empty() {
            return Ok(None);
        }
        Ok(Some(password))
    }
}

#[derive(Clone)]
struct AppState {
    config: AuthServerConfig,
    tailscale: tailscale::TailscaleBridgeManager,
}

#[derive(Debug, Deserialize)]
struct TailnetDiscoveryQuery {
    email: String,
}

type AppResult<T> = Result<T, (StatusCode, String)>;

pub async fn serve() -> Result<()> {
    serve_with_config(AuthServerConfig::from_env()).await
}

pub async fn serve_with_config(config: AuthServerConfig) -> Result<()> {
    db::init_db(&config.db_path)?;
    if let Some(password) = config.bootstrap_password()? {
        db::ensure_local_identity(
            &config.db_path,
            &config.bootstrap.username,
            &config.bootstrap.email,
            &config.bootstrap.display_name,
            &password,
        )?;
    }

    let app = build_router(config.clone());
    let listener = tokio::net::TcpListener::bind(&config.listen).await?;
    log::info!("Starting auth server on {}", config.listen);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

pub fn build_router(config: AuthServerConfig) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/device/new", post(device_new))
        .route("/v1/auth/login", post(login_local))
        .route("/v1/control/register", post(control_register))
        .route("/v1/control/map", post(control_map))
        .route("/v1/tailnet/discover", get(tailnet_discover))
        .route("/v1/tailscale/login/start", post(tailscale_login_start))
        .route("/v1/tailscale/login/:session_id", get(tailscale_login_status))
        .with_state(AppState {
            config,
            tailscale: tailscale::TailscaleBridgeManager::default(),
        })
}

async fn login_local(
    State(state): State<AppState>,
    Json(request): Json<LocalAuthRequest>,
) -> AppResult<Json<LocalAuthResponse>> {
    let db_path = state.config.db_path.clone();
    blocking(move || db::authenticate_local(&db_path, &request.identifier, &request.password))
        .await?
        .map(Json)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "invalid credentials".to_owned()))
}

async fn control_register(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> AppResult<Json<RegisterResponse>> {
    let token = bearer_token(&headers)?;
    let db_path = state.config.db_path.clone();
    let user = blocking({
        let db_path = db_path.clone();
        let token = token.clone();
        move || db::user_for_session(&db_path, &token)
    })
    .await?
    .ok_or_else(|| (StatusCode::UNAUTHORIZED, "unknown session".to_owned()))?;

    let response_user = user.profile.clone();
    let node = blocking(move || db::upsert_node(&db_path, &user, &request)).await?;
    Ok(Json(RegisterResponse {
        user: response_user,
        machine_authorized: node.machine_authorized,
        node_key_expired: node.node_key_expired,
        auth_url: None,
        error: None,
        node,
    }))
}

async fn control_map(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<MapRequest>,
) -> AppResult<Json<MapResponse>> {
    let token = bearer_token(&headers)?;
    let db_path = state.config.db_path.clone();
    let domain = state.config.tailnet_domain.clone();
    let user = blocking({
        let db_path = db_path.clone();
        let token = token.clone();
        move || db::user_for_session(&db_path, &token)
    })
    .await?
    .ok_or_else(|| (StatusCode::UNAUTHORIZED, "unknown session".to_owned()))?;

    let response = blocking(move || db::map_for_node(&db_path, &user, &request, &domain)).await?;
    Ok(Json(response))
}

async fn tailnet_discover(
    Query(query): Query<TailnetDiscoveryQuery>,
) -> AppResult<Json<TailnetDiscovery>> {
    if query.email.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "email is required".to_owned()));
    }

    let discovery = discovery::discover_tailnet(&query.email)
        .await
        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
    Ok(Json(discovery))
}

async fn tailscale_login_start(
    State(state): State<AppState>,
    Json(request): Json<tailscale::TailscaleLoginStartRequest>,
) -> AppResult<Json<tailscale::TailscaleLoginStartResponse>> {
    let response = state
        .tailscale
        .start_login(request)
        .await
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn tailscale_login_status(
    AxumPath(session_id): AxumPath<String>,
    State(state): State<AppState>,
) -> AppResult<Json<tailscale::TailscaleLoginStatus>> {
    state
        .tailscale
        .status(&session_id)
        .await
        .map_err(internal_error)?
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "unknown tailscale login session".to_owned()))
}

async fn healthz() -> impl IntoResponse {
    StatusCode::OK
}

async fn device_new() -> impl IntoResponse {
    StatusCode::OK
}

async fn blocking<F, T>(work: F) -> AppResult<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .map_err(internal_error)
}

fn internal_error(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn bearer_token(headers: &HeaderMap) -> AppResult<String> {
    let value = headers.get(AUTHORIZATION).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            "missing authorization header".to_owned(),
        )
    })?;
    let value = value.to_str().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "invalid authorization header".to_owned(),
        )
    })?;
    value
        .strip_prefix("Bearer ")
        .map(ToOwned::to_owned)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "expected bearer token".to_owned()))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tempfile::tempdir;
    use tower::ServiceExt;

    #[tokio::test]
    async fn login_register_and_map_round_trip() -> Result<()> {
        let dir = tempdir()?;
        let password_file = dir.path().join("bootstrap-password.txt");
        std::fs::write(&password_file, "bootstrap-pass\n")?;
        let db_path = dir.path().join("server.sqlite3");
        let config = AuthServerConfig {
            listen: "127.0.0.1:0".to_owned(),
            db_path: db_path.to_string_lossy().to_string(),
            tailnet_domain: "burrow.net".to_owned(),
            bootstrap: BootstrapIdentity {
                password_file: password_file.to_string_lossy().to_string(),
                ..BootstrapIdentity::default()
            },
        };

        db::init_db(&config.db_path)?;
        let password = config.bootstrap_password()?.expect("bootstrap password");
        db::ensure_local_identity(
            &config.db_path,
            &config.bootstrap.username,
            &config.bootstrap.email,
            &config.bootstrap.display_name,
            &password,
        )?;

        let app = build_router(config);

        let response = app
            .clone()
            .oneshot(
                Request::post("/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&LocalAuthRequest {
                        identifier: "contact".to_owned(),
                        password: "bootstrap-pass".to_owned(),
                    })?))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let login: LocalAuthResponse =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await?)?;

        let response = app
            .clone()
            .oneshot(
                Request::post("/v1/control/register")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", login.access_token))
                    .body(Body::from(serde_json::to_vec(&RegisterRequest {
                        node_key: "nodekey:1234".to_owned(),
                        machine_key: Some("machinekey:1234".to_owned()),
                        addresses: vec!["100.64.0.10/32".to_owned()],
                        endpoints: vec!["198.51.100.10:41641".to_owned()],
                        hostinfo: Some(crate::control::Hostinfo {
                            hostname: Some("devbox".to_owned()),
                            os: Some("linux".to_owned()),
                            os_version: Some("6.13".to_owned()),
                            services: vec!["ssh".to_owned()],
                            request_tags: vec!["tag:dev".to_owned()],
                        }),
                        ..RegisterRequest::default()
                    })?))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::post("/v1/control/map")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", login.access_token))
                    .body(Body::from(serde_json::to_vec(&MapRequest {
                        node_key: "nodekey:1234".to_owned(),
                        stream: true,
                        endpoints: vec!["198.51.100.10:41641".to_owned()],
                        ..MapRequest::default()
                    })?))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        let map: MapResponse =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await?)?;
        assert_eq!(map.domain, "burrow.net");
        assert_eq!(map.node.name, "devbox");
        assert!(map.dns.expect("dns").magic_dns);
        Ok(())
    }

    #[tokio::test]
    async fn tailnet_discover_requires_email() -> Result<()> {
        let app = build_router(AuthServerConfig::default());
        let response = app
            .oneshot(
                Request::get("/v1/tailnet/discover?email=")
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }
}
