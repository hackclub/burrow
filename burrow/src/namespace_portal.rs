#![cfg(target_os = "linux")]

use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, Context, Result};
use axum::{
    extract::{Query, State},
    http::{
        header::{COOKIE, LOCATION, SET_COOKIE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use reqwest::Url;
use ring::digest::{digest, SHA256};
use serde::Deserialize;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::Mutex,
};

const SESSION_COOKIE: &str = "burrow_namespace_portal_session";
const OIDC_TIMEOUT: Duration = Duration::from_secs(600);
const AUTH_CHECK_DURATION: &str = "10m";

#[derive(Clone, Debug)]
pub struct NamespacePortalConfig {
    pub listen: String,
    pub public_base_url: String,
    pub oidc_discovery_url: String,
    pub oidc_client_id: String,
    pub oidc_client_secret: Option<String>,
    pub allowed_group: String,
    pub nsc_bin: String,
    pub nsc_state_dir: PathBuf,
    pub token_output_path: PathBuf,
}

impl Default for NamespacePortalConfig {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:9080".to_owned(),
            public_base_url: "https://nsc.burrow.net".to_owned(),
            oidc_discovery_url:
                "https://auth.burrow.net/application/o/namespace/.well-known/openid-configuration"
                    .to_owned(),
            oidc_client_id: "nsc.burrow.net".to_owned(),
            oidc_client_secret: None,
            allowed_group: "burrow-admins".to_owned(),
            nsc_bin: "nsc".to_owned(),
            nsc_state_dir: PathBuf::from("/var/lib/burrow/namespace-portal/nsc"),
            token_output_path: PathBuf::from("/var/lib/burrow/intake/forgejo_nsc_token.txt"),
        }
    }
}

impl NamespacePortalConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(value) = env::var("BURROW_NAMESPACE_PORTAL_LISTEN") {
            config.listen = value;
        }
        if let Ok(value) = env::var("BURROW_NAMESPACE_PORTAL_BASE_URL") {
            config.public_base_url = value;
        }
        if let Ok(value) = env::var("BURROW_NAMESPACE_PORTAL_OIDC_DISCOVERY_URL") {
            config.oidc_discovery_url = value;
        }
        if let Ok(value) = env::var("BURROW_NAMESPACE_PORTAL_OIDC_CLIENT_ID") {
            config.oidc_client_id = value;
        }
        if let Ok(value) = env::var("BURROW_NAMESPACE_PORTAL_OIDC_CLIENT_SECRET") {
            let value = value.trim().to_owned();
            if !value.is_empty() {
                config.oidc_client_secret = Some(value);
            }
        }
        if let Ok(value) = env::var("BURROW_NAMESPACE_PORTAL_ALLOWED_GROUP") {
            config.allowed_group = value;
        }
        if let Ok(value) = env::var("BURROW_NAMESPACE_PORTAL_NSC_BIN") {
            config.nsc_bin = value;
        }
        if let Ok(value) = env::var("BURROW_NAMESPACE_PORTAL_NSC_STATE_DIR") {
            config.nsc_state_dir = PathBuf::from(value);
        }
        if let Ok(value) = env::var("BURROW_NAMESPACE_PORTAL_TOKEN_OUTPUT_PATH") {
            config.token_output_path = PathBuf::from(value);
        }
        config
    }

    fn callback_url(&self) -> Result<String> {
        let mut url = Url::parse(&self.public_base_url)
            .with_context(|| format!("invalid public base url {}", self.public_base_url))?;
        url.set_path("/oauth/callback");
        url.set_query(None);
        Ok(url.to_string())
    }

    fn ensure_paths(&self) -> Result<()> {
        fs::create_dir_all(&self.nsc_state_dir).with_context(|| {
            format!(
                "failed to create namespace portal state dir {}",
                self.nsc_state_dir.display()
            )
        })?;
        if let Some(parent) = self.token_output_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create token output dir {}", parent.display())
            })?;
        }
        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    config: NamespacePortalConfig,
    client: reqwest::Client,
    oidc: OidcDiscovery,
    pending_logins: Arc<Mutex<HashMap<String, PendingOidcLogin>>>,
    sessions: Arc<Mutex<HashMap<String, PortalSession>>>,
    namespace: NamespaceSessionManager,
}

#[derive(Clone, Debug, Deserialize)]
struct OidcDiscovery {
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
}

#[derive(Clone, Debug)]
struct PendingOidcLogin {
    verifier: String,
    expires_at: Instant,
}

#[derive(Clone, Debug)]
struct PortalSession {
    email: String,
    display_name: String,
    groups: Vec<String>,
    issued_at: Instant,
}

#[derive(Debug, Deserialize)]
struct OidcCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    #[serde(default)]
    email: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    preferred_username: String,
    #[serde(default)]
    groups: Vec<String>,
}

#[derive(Clone)]
struct NamespaceSessionManager {
    config: NamespacePortalConfig,
    state: Arc<Mutex<NamespacePortalState>>,
}

#[derive(Clone, Debug, Default)]
struct NamespacePortalState {
    active_login: Option<ActiveNamespaceLogin>,
    last_error: Option<String>,
}

#[derive(Clone, Debug)]
struct ActiveNamespaceLogin {
    login_url: String,
}

#[derive(Clone, Debug)]
struct NamespaceStatus {
    linked: bool,
    login_url: Option<String>,
    last_error: Option<String>,
    token_present: bool,
}

pub async fn serve() -> Result<()> {
    serve_with_config(NamespacePortalConfig::from_env()).await
}

pub async fn refresh_token_once() -> Result<()> {
    let config = NamespacePortalConfig::from_env();
    config.ensure_paths()?;
    NamespaceSessionManager::new(config).refresh_token().await
}

pub async fn serve_with_config(config: NamespacePortalConfig) -> Result<()> {
    config.ensure_paths()?;
    let oidc = fetch_oidc_discovery(&config.oidc_discovery_url).await?;
    let listen = config.listen.clone();
    let app = Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .route("/login", get(oidc_login))
        .route("/logout", post(logout))
        .route("/oauth/callback", get(oidc_callback))
        .route("/namespace/link/start", post(namespace_link_start))
        .route("/namespace/token/refresh", post(namespace_token_refresh))
        .with_state(AppState {
            config: config.clone(),
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            oidc,
            pending_logins: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            namespace: NamespaceSessionManager::new(config),
        });

    let listener = tokio::net::TcpListener::bind(&listen).await?;
    log::info!("Starting Namespace portal on {}", listen);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn fetch_oidc_discovery(discovery_url: &str) -> Result<OidcDiscovery> {
    reqwest::Client::new()
        .get(discovery_url)
        .send()
        .await
        .with_context(|| format!("failed to fetch oidc discovery {}", discovery_url))?
        .error_for_status()
        .with_context(|| format!("oidc discovery returned non-success {}", discovery_url))?
        .json()
        .await
        .context("failed to decode oidc discovery document")
}

async fn healthz() -> impl IntoResponse {
    StatusCode::OK
}

async fn index(State(state): State<AppState>, headers: HeaderMap) -> Response {
    match current_session(&state, &headers).await {
        Ok(Some(session)) => {
            let namespace_status = match state.namespace.status().await {
                Ok(status) => status,
                Err(err) => NamespaceStatus {
                    linked: false,
                    login_url: None,
                    last_error: Some(err.to_string()),
                    token_present: false,
                },
            };
            Html(render_dashboard(&state.config, &session, &namespace_status)).into_response()
        }
        Ok(None) => Html(render_login_page()).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(render_error_page(&format!("session lookup failed: {err}"))),
        )
            .into_response(),
    }
}

async fn oidc_login(State(state): State<AppState>) -> Result<Redirect, (StatusCode, String)> {
    prune_pending(&state).await;
    let state_token = random_url_token(32);
    let verifier = random_url_token(48);
    let challenge = pkce_challenge(&verifier);
    let callback_url = state.config.callback_url().map_err(internal_error)?;

    state.pending_logins.lock().await.insert(
        state_token.clone(),
        PendingOidcLogin {
            verifier,
            expires_at: Instant::now() + OIDC_TIMEOUT,
        },
    );

    let mut url = Url::parse(&state.oidc.authorization_endpoint).map_err(internal_error)?;
    url.query_pairs_mut()
        .append_pair("client_id", &state.config.oidc_client_id)
        .append_pair("response_type", "code")
        .append_pair("scope", "openid profile email groups")
        .append_pair("redirect_uri", &callback_url)
        .append_pair("state", &state_token)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(Redirect::to(url.as_str()))
}

async fn oidc_callback(
    State(state): State<AppState>,
    Query(query): Query<OidcCallbackQuery>,
) -> Result<Response, (StatusCode, String)> {
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("oidc login failed: {error} {description}")
                .trim()
                .to_owned(),
        ));
    }

    let code = query
        .code
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing oidc code".to_owned()))?;
    let state_token = query
        .state
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing oidc state".to_owned()))?;

    let verifier = {
        let mut pending = state.pending_logins.lock().await;
        let Some(login) = pending.remove(&state_token) else {
            return Err((StatusCode::BAD_REQUEST, "unknown oidc state".to_owned()));
        };
        if login.expires_at <= Instant::now() {
            return Err((StatusCode::BAD_REQUEST, "expired oidc state".to_owned()));
        }
        login.verifier
    };

    let callback_url = state.config.callback_url().map_err(internal_error)?;

    let mut params = vec![
        ("grant_type", "authorization_code".to_owned()),
        ("code", code),
        ("client_id", state.config.oidc_client_id.clone()),
        ("redirect_uri", callback_url),
        ("code_verifier", verifier),
    ];
    if let Some(secret) = &state.config.oidc_client_secret {
        params.push(("client_secret", secret.clone()));
    }

    let token = state
        .client
        .post(&state.oidc.token_endpoint)
        .form(&params)
        .send()
        .await
        .context("failed to exchange oidc code")
        .map_err(internal_error)?
        .error_for_status()
        .context("oidc token endpoint returned non-success")
        .map_err(internal_error)?
        .json::<TokenResponse>()
        .await
        .context("failed to decode oidc token response")
        .map_err(internal_error)?;

    let userinfo = state
        .client
        .get(&state.oidc.userinfo_endpoint)
        .bearer_auth(&token.access_token)
        .send()
        .await
        .context("failed to fetch oidc userinfo")
        .map_err(internal_error)?
        .error_for_status()
        .context("oidc userinfo returned non-success")
        .map_err(internal_error)?
        .json::<UserInfo>()
        .await
        .context("failed to decode oidc userinfo")
        .map_err(internal_error)?;

    if !userinfo
        .groups
        .iter()
        .any(|group| group == &state.config.allowed_group)
    {
        return Err((
            StatusCode::FORBIDDEN,
            format!(
                "authenticated user is not in required group {}",
                state.config.allowed_group
            ),
        ));
    }

    let session_id = random_url_token(32);
    state.sessions.lock().await.insert(
        session_id.clone(),
        PortalSession {
            email: userinfo.email.clone(),
            display_name: display_name(&userinfo),
            groups: userinfo.groups,
            issued_at: Instant::now(),
        },
    );

    let mut response = Redirect::to("/").into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&session_cookie_value(&session_id)).map_err(internal_error)?,
    );
    Ok(response)
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    if let Some(session_id) = session_cookie(&headers) {
        state.sessions.lock().await.remove(&session_id);
    }
    let mut response = Redirect::to("/").into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_static(
            "burrow_namespace_portal_session=; Path=/; Max-Age=0; HttpOnly; Secure; SameSite=Lax",
        ),
    );
    Ok(response)
}

async fn namespace_link_start(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Redirect, (StatusCode, String)> {
    require_session(&state, &headers).await?;
    state
        .namespace
        .start_login()
        .await
        .map_err(internal_error)?;
    Ok(Redirect::to("/"))
}

async fn namespace_token_refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Redirect, (StatusCode, String)> {
    require_session(&state, &headers).await?;
    state
        .namespace
        .refresh_token()
        .await
        .map_err(internal_error)?;
    Ok(Redirect::to("/"))
}

fn render_login_page() -> String {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Burrow Namespace Portal</title>
  <style>
    body { font-family: ui-sans-serif, system-ui, sans-serif; background: #0b1020; color: #eef3ff; margin: 0; }
    main { max-width: 32rem; margin: 8rem auto; padding: 2rem; background: rgba(19, 28, 52, 0.82); border-radius: 1.5rem; box-shadow: 0 24px 64px rgba(0,0,0,0.28); }
    h1 { margin-top: 0; font-size: 1.8rem; }
    p { color: #c3cee8; line-height: 1.5; }
    a.button { display: inline-block; margin-top: 1rem; padding: 0.85rem 1.25rem; border-radius: 999px; text-decoration: none; color: #08201e; background: linear-gradient(135deg, #6df2d4, #7bd1ff); font-weight: 700; }
  </style>
</head>
<body>
  <main>
    <h1>Burrow Namespace Portal</h1>
    <p>Authenticate with <strong>burrow.net</strong> to manage the dedicated Namespace session that backs Forgejo NSC automation.</p>
    <a class="button" href="/login">Sign in with burrow.net</a>
  </main>
</body>
</html>"#
        .to_owned()
}

fn render_dashboard(
    config: &NamespacePortalConfig,
    session: &PortalSession,
    status: &NamespaceStatus,
) -> String {
    let refresh = if status.login_url.is_some() {
        r#"<meta http-equiv="refresh" content="3">"#
    } else {
        ""
    };
    let login_action = if let Some(url) = &status.login_url {
        format!(
            "<section class=\"card\"><h2>Namespace Login In Progress</h2><p>Open the live Namespace URL below with the dedicated Burrow account. This page will refresh automatically until the server-side session is ready.</p><p><a class=\"link\" href=\"{}\">Open Namespace Login</a></p></section>",
            escape_html(url)
        )
    } else if status.linked {
        "<section class=\"card\"><h2>Namespace Linked</h2><p>The forge-owned NSC session is authenticated and ready to mint runner tokens.</p></section>".to_owned()
    } else {
        "<section class=\"card\"><h2>Namespace Not Linked</h2><p>Start a server-side Namespace login. The portal will produce a Namespace URL, and completing that browser flow will authenticate the forge-owned NSC state directory.</p></section>".to_owned()
    };
    let error = status
        .last_error
        .as_ref()
        .map(|error| format!("<p class=\"error\">{}</p>", escape_html(error)))
        .unwrap_or_default();
    let token_state = if status.token_present {
        "present"
    } else {
        "missing"
    };
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Burrow Namespace Portal</title>
  {refresh}
  <style>
    body {{ font-family: ui-sans-serif, system-ui, sans-serif; background: linear-gradient(180deg, #f4f7ff, #e9eefc); color: #1b2747; margin: 0; }}
    main {{ max-width: 46rem; margin: 3rem auto; padding: 0 1rem 3rem; }}
    header {{ display: flex; align-items: center; justify-content: space-between; gap: 1rem; margin-bottom: 1rem; }}
    h1 {{ margin: 0; font-size: 1.8rem; }}
    .subtle {{ color: #66718d; }}
    .card {{ background: rgba(255,255,255,0.86); border-radius: 1.4rem; box-shadow: 0 18px 44px rgba(53, 73, 120, 0.12); padding: 1.25rem 1.35rem; margin-top: 1rem; }}
    .actions {{ display: flex; flex-wrap: wrap; gap: 0.75rem; margin-top: 1rem; }}
    button {{ border: none; border-radius: 999px; padding: 0.85rem 1.2rem; font: inherit; font-weight: 700; background: linear-gradient(135deg, #6df2d4, #7bd1ff); color: #08201e; cursor: pointer; }}
    .secondary {{ background: #eef2fb; color: #30405f; }}
    .link {{ color: #0f63ff; font-weight: 700; text-decoration: none; }}
    .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr)); gap: 0.9rem; }}
    .metric {{ background: rgba(237, 242, 255, 0.95); border-radius: 1rem; padding: 0.9rem 1rem; }}
    .label {{ font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.08em; color: #7380a1; }}
    .value {{ margin-top: 0.35rem; font-size: 1rem; font-weight: 700; }}
    .error {{ color: #a81f43; font-weight: 600; }}
    form {{ margin: 0; }}
  </style>
</head>
<body>
  <main>
    <header>
      <div>
        <h1>Burrow Namespace Portal</h1>
        <p class="subtle">Signed in as {email}. This page controls the forge-owned NSC session and token material for Forgejo Namespace runners.</p>
      </div>
      <form action="/logout" method="post"><button class="secondary" type="submit">Sign Out</button></form>
    </header>

    <section class="card">
      <div class="grid">
        <div class="metric"><div class="label">burrow.net identity</div><div class="value">{identity}</div></div>
        <div class="metric"><div class="label">required group</div><div class="value">{group}</div></div>
        <div class="metric"><div class="label">NSC token file</div><div class="value">{token_path}</div></div>
        <div class="metric"><div class="label">current token</div><div class="value">{token_state}</div></div>
      </div>
    </section>

    {login_action}
    {error}

    <section class="card">
      <h2>Actions</h2>
      <div class="actions">
        <form action="/namespace/link/start" method="post"><button type="submit">Link Namespace</button></form>
        <form action="/namespace/token/refresh" method="post"><button class="secondary" type="submit">Rotate NSC Token</button></form>
      </div>
    </section>
  </main>
</body>
</html>"#,
        refresh = refresh,
        email = escape_html(&session.email),
        identity = escape_html(&session.display_name),
        group = escape_html(&config.allowed_group),
        token_path = escape_html(&config.token_output_path.display().to_string()),
        token_state = token_state,
        login_action = login_action,
        error = error,
    )
}

fn render_error_page(message: &str) -> String {
    format!(
        r#"<!doctype html><html lang="en"><body><main><h1>Namespace Portal Error</h1><p>{}</p></main></body></html>"#,
        escape_html(message)
    )
}

fn display_name(userinfo: &UserInfo) -> String {
    if !userinfo.name.trim().is_empty() {
        return userinfo.name.trim().to_owned();
    }
    if !userinfo.preferred_username.trim().is_empty() {
        return userinfo.preferred_username.trim().to_owned();
    }
    userinfo.email.clone()
}

async fn current_session(state: &AppState, headers: &HeaderMap) -> Result<Option<PortalSession>> {
    let Some(session_id) = session_cookie(headers) else {
        return Ok(None);
    };
    Ok(state.sessions.lock().await.get(&session_id).cloned())
}

async fn require_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<PortalSession, (StatusCode, String)> {
    current_session(state, headers)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "sign-in required".to_owned()))
}

async fn prune_pending(state: &AppState) {
    state
        .pending_logins
        .lock()
        .await
        .retain(|_, login| login.expires_at > Instant::now());
}

fn session_cookie(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(COOKIE)?.to_str().ok()?;
    for pair in cookie_header.split(';') {
        let mut parts = pair.trim().splitn(2, '=');
        let name = parts.next()?.trim();
        let value = parts.next()?.trim();
        if name == SESSION_COOKIE && !value.is_empty() {
            return Some(value.to_owned());
        }
    }
    None
}

fn session_cookie_value(session_id: &str) -> String {
    format!("{SESSION_COOKIE}={session_id}; Path=/; HttpOnly; Secure; SameSite=Lax")
}

fn random_url_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = digest(&SHA256, verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest.as_ref())
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn internal_error(err: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

impl NamespaceSessionManager {
    fn new(config: NamespacePortalConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(NamespacePortalState::default())),
        }
    }

    async fn status(&self) -> Result<NamespaceStatus> {
        let linked = self.check_login().await.is_ok();
        let state = self.state.lock().await.clone();
        let token_present = tokio::fs::metadata(&self.config.token_output_path)
            .await
            .is_ok();
        Ok(NamespaceStatus {
            linked,
            login_url: state.active_login.map(|login| login.login_url),
            last_error: state.last_error,
            token_present,
        })
    }

    async fn start_login(&self) -> Result<String> {
        if self.check_login().await.is_ok() {
            self.refresh_token().await?;
            return Ok("already linked".to_owned());
        }

        {
            let state = self.state.lock().await;
            if let Some(active) = &state.active_login {
                return Ok(active.login_url.clone());
            }
        }

        self.config.ensure_paths()?;
        let mut command = self.base_command();
        command
            .args(["auth", "login", "--browser=false"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().context("failed to spawn nsc auth login")?;
        let stdout = child
            .stdout
            .take()
            .context("nsc auth login stdout was not piped")?;
        let mut lines = BufReader::new(stdout).lines();
        let mut login_url = None;
        while let Some(line) = lines.next_line().await? {
            if let Some(candidate) = extract_namespace_login_url(&line) {
                login_url = Some(candidate);
                break;
            }
        }

        let login_url = login_url
            .ok_or_else(|| anyhow!("nsc auth login did not emit a Namespace login URL"))?;
        {
            let mut state = self.state.lock().await;
            state.active_login = Some(ActiveNamespaceLogin { login_url: login_url.clone() });
            state.last_error = None;
        }

        let manager = self.clone();
        tokio::spawn(async move {
            let outcome = child.wait().await;
            let mut state = manager.state.lock().await;
            state.active_login = None;
            match outcome {
                Ok(status) if status.success() => {
                    drop(state);
                    if let Err(err) = manager.refresh_token().await {
                        manager.state.lock().await.last_error = Some(format!(
                            "Namespace login finished, but token refresh failed: {err}"
                        ));
                    }
                }
                Ok(status) => {
                    state.last_error = Some(format!(
                        "Namespace login command exited with status {}",
                        status
                    ));
                }
                Err(err) => {
                    state.last_error = Some(format!("Namespace login command failed: {err}"));
                }
            }
        });

        Ok(login_url)
    }

    async fn refresh_token(&self) -> Result<()> {
        self.config.ensure_paths()?;
        self.check_login().await?;
        let mut command = self.base_command();
        command.args([
            "auth",
            "generate-dev-token",
            "--output_to",
            self.config
                .token_output_path
                .to_str()
                .ok_or_else(|| anyhow!("token output path is not valid UTF-8"))?,
        ]);
        let output = command
            .output()
            .await
            .context("failed to run nsc token refresh")?;
        if !output.status.success() {
            bail!(
                "nsc auth generate-dev-token failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::PermissionsExt;

            let perms = fs::Permissions::from_mode(0o440);
            fs::set_permissions(&self.config.token_output_path, perms).with_context(|| {
                format!(
                    "failed to set permissions on {}",
                    self.config.token_output_path.display()
                )
            })?;
        }
        self.state.lock().await.last_error = None;
        Ok(())
    }

    async fn check_login(&self) -> Result<()> {
        let mut command = self.base_command();
        command.args(["auth", "check-login", "--duration", AUTH_CHECK_DURATION]);
        let output = command
            .output()
            .await
            .context("failed to run nsc auth check-login")?;
        if output.status.success() {
            return Ok(());
        }
        bail!("{}", String::from_utf8_lossy(&output.stderr).trim());
    }

    fn base_command(&self) -> Command {
        let mut command = Command::new(&self.config.nsc_bin);
        let home = self.config.nsc_state_dir.join("home");
        let data = self.config.nsc_state_dir.join("data");
        let cache = self.config.nsc_state_dir.join("cache");
        let config = self.config.nsc_state_dir.join("config");
        let _ = fs::create_dir_all(&home);
        let _ = fs::create_dir_all(&data);
        let _ = fs::create_dir_all(&cache);
        let _ = fs::create_dir_all(&config);
        command
            .env("HOME", &home)
            .env("XDG_DATA_HOME", &data)
            .env("XDG_CACHE_HOME", &cache)
            .env("XDG_CONFIG_HOME", &config);
        command
    }
}

fn extract_namespace_login_url(line: &str) -> Option<String> {
    line.split_whitespace()
        .find(|token| token.starts_with("https://"))
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_namespace_login_url_from_output() {
        let url = extract_namespace_login_url(
            "  https://cloud.namespace.so/login/workspace?id=p0cl4ik19c4c473u14tvc3vq2o",
        );
        assert_eq!(
            url.as_deref(),
            Some("https://cloud.namespace.so/login/workspace?id=p0cl4ik19c4c473u14tvc3vq2o")
        );
    }

    #[test]
    fn pkce_challenge_is_stable() {
        assert_eq!(
            pkce_challenge("hello"),
            "LPJNul-wow4m6DsqxbninhsWHlwfp0JecwQzYpOLmCQ"
        );
    }

    #[test]
    fn parses_session_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static(
                "something=else; burrow_namespace_portal_session=session123; another=value",
            ),
        );
        assert_eq!(session_cookie(&headers).as_deref(), Some("session123"));
    }
}
