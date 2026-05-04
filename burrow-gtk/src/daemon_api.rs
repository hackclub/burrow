use anyhow::{anyhow, Context, Result};
use burrow::{
    control::{TailnetConfig, TailnetProvider},
    grpc_defs::{
        Empty, Network, NetworkType, State, TailnetDiscoverRequest, TailnetLoginCancelRequest,
        TailnetLoginStartRequest, TailnetLoginStatusRequest, TailnetProbeRequest,
    },
    BurrowClient,
};
use std::{path::PathBuf, sync::OnceLock};
use tokio::time::{timeout, Duration};

const RPC_TIMEOUT: Duration = Duration::from_secs(3);
const MANAGED_TAILSCALE_AUTHORITY: &str = "https://controlplane.tailscale.com";
static EMBEDDED_DAEMON_STARTED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelState {
    Running,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct NetworkSummary {
    pub id: i32,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct TailnetDiscovery {
    pub authority: String,
    pub managed: bool,
    pub oidc_issuer: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TailnetProbe {
    pub summary: String,
    pub detail: Option<String>,
    pub status_code: i32,
}

#[derive(Debug, Clone)]
pub struct TailnetLoginStatus {
    pub session_id: String,
    pub backend_state: String,
    pub auth_url: Option<String>,
    pub running: bool,
    pub needs_login: bool,
    pub tailnet_name: Option<String>,
    pub self_dns_name: Option<String>,
    pub tailnet_ips: Vec<String>,
    pub health: Vec<String>,
}

pub fn default_tailnet_authority() -> &'static str {
    MANAGED_TAILSCALE_AUTHORITY
}

pub fn configure_client_paths() -> Result<()> {
    if std::env::var_os("BURROW_SOCKET_PATH").is_none() {
        std::env::set_var("BURROW_SOCKET_PATH", default_socket_path()?);
    }
    Ok(())
}

pub async fn ensure_daemon() -> Result<()> {
    configure_client_paths()?;
    if daemon_available().await {
        return Ok(());
    }

    let socket_path = socket_path()?;
    let db_path = database_path()?;
    ensure_parent(&socket_path)?;
    ensure_parent(&db_path)?;

    if EMBEDDED_DAEMON_STARTED.get().is_none() {
        tokio::task::spawn_blocking(move || {
            burrow::spawn_in_process_with_paths(Some(socket_path), Some(db_path));
        })
        .await
        .context("failed to join embedded daemon startup")?;
        let _ = EMBEDDED_DAEMON_STARTED.set(());
    }

    tunnel_state()
        .await
        .map(|_| ())
        .context("Burrow daemon started but did not accept tunnel status RPCs")
}

pub fn infer_tailnet_provider(authority: &str) -> TailnetProvider {
    let normalized = authority.trim().trim_end_matches('/').to_ascii_lowercase();
    if normalized == "controlplane.tailscale.com"
        || normalized == "http://controlplane.tailscale.com"
        || normalized == MANAGED_TAILSCALE_AUTHORITY
    {
        TailnetProvider::Tailscale
    } else {
        TailnetProvider::Headscale
    }
}

pub async fn daemon_available() -> bool {
    tunnel_state().await.is_ok()
}

fn socket_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("BURROW_SOCKET_PATH") {
        return Ok(PathBuf::from(path));
    }
    default_socket_path()
}

fn default_socket_path() -> Result<PathBuf> {
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return Ok(PathBuf::from(runtime_dir).join("burrow.sock"));
    }
    let uid = std::env::var("UID").unwrap_or_else(|_| "1000".to_owned());
    Ok(PathBuf::from(format!("/tmp/burrow-{uid}.sock")))
}

fn database_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("BURROW_DB_PATH") {
        return Ok(PathBuf::from(path));
    }
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(data_home).join("burrow").join("burrow.db"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("burrow")
            .join("burrow.db"));
    }
    Ok(std::env::temp_dir().join("burrow.db"))
}

fn ensure_parent(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(())
}

pub async fn tunnel_state() -> Result<TunnelState> {
    let mut client = BurrowClient::from_uds().await?;
    let mut stream = timeout(RPC_TIMEOUT, client.tunnel_client.tunnel_status(Empty {}))
        .await
        .context("timed out connecting to Burrow daemon")??
        .into_inner();
    let status = timeout(RPC_TIMEOUT, stream.message())
        .await
        .context("timed out reading Burrow tunnel status")??
        .context("Burrow daemon ended the status stream without a state")?;
    Ok(match status.state() {
        State::Running => TunnelState::Running,
        State::Stopped => TunnelState::Stopped,
    })
}

pub async fn start_tunnel() -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    timeout(RPC_TIMEOUT, client.tunnel_client.tunnel_start(Empty {}))
        .await
        .context("timed out starting Burrow tunnel")??;
    Ok(())
}

pub async fn stop_tunnel() -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    timeout(RPC_TIMEOUT, client.tunnel_client.tunnel_stop(Empty {}))
        .await
        .context("timed out stopping Burrow tunnel")??;
    Ok(())
}

pub async fn list_networks() -> Result<Vec<NetworkSummary>> {
    let mut client = BurrowClient::from_uds().await?;
    let mut stream = timeout(RPC_TIMEOUT, client.networks_client.network_list(Empty {}))
        .await
        .context("timed out connecting to Burrow network list")??
        .into_inner();
    let response = timeout(RPC_TIMEOUT, stream.message())
        .await
        .context("timed out reading Burrow network list")??
        .context("Burrow daemon ended the network stream without a snapshot")?;
    Ok(response.network.iter().map(summarize_network).collect())
}

pub async fn add_wireguard(config: String) -> Result<i32> {
    add_network(NetworkType::WireGuard, config.into_bytes()).await
}

pub async fn add_tailnet(
    authority: String,
    account: String,
    identity: String,
    hostname: Option<String>,
    tailnet: Option<String>,
) -> Result<i32> {
    let provider = infer_tailnet_provider(&authority);
    let config = TailnetConfig {
        provider,
        authority: Some(authority),
        account: Some(account),
        identity: Some(identity),
        hostname,
        tailnet,
    };
    let payload = serde_json::to_vec_pretty(&config)?;
    add_network(NetworkType::Tailnet, payload).await
}

pub async fn discover_tailnet(email: String) -> Result<TailnetDiscovery> {
    let mut client = BurrowClient::from_uds().await?;
    let response = timeout(
        RPC_TIMEOUT,
        client
            .tailnet_client
            .discover(TailnetDiscoverRequest { email }),
    )
    .await
    .context("timed out discovering Tailnet authority")??
    .into_inner();

    Ok(TailnetDiscovery {
        authority: response.authority,
        managed: response.managed,
        oidc_issuer: optional(response.oidc_issuer),
    })
}

pub async fn probe_tailnet(authority: String) -> Result<TailnetProbe> {
    let mut client = BurrowClient::from_uds().await?;
    let response = timeout(
        RPC_TIMEOUT,
        client
            .tailnet_client
            .probe(TailnetProbeRequest { authority }),
    )
    .await
    .context("timed out probing Tailnet authority")??
    .into_inner();

    Ok(TailnetProbe {
        summary: response.summary,
        detail: optional(response.detail),
        status_code: response.status_code,
    })
}

pub async fn start_tailnet_login(
    authority: String,
    account_name: String,
    identity_name: String,
    hostname: Option<String>,
) -> Result<TailnetLoginStatus> {
    let mut client = BurrowClient::from_uds().await?;
    let response = timeout(
        RPC_TIMEOUT,
        client.tailnet_client.login_start(TailnetLoginStartRequest {
            account_name,
            identity_name,
            hostname: hostname.unwrap_or_default(),
            authority,
        }),
    )
    .await
    .context("timed out starting Tailnet sign-in")??
    .into_inner();
    Ok(decode_tailnet_status(response))
}

pub async fn tailnet_login_status(session_id: String) -> Result<TailnetLoginStatus> {
    let mut client = BurrowClient::from_uds().await?;
    let response = timeout(
        RPC_TIMEOUT,
        client
            .tailnet_client
            .login_status(TailnetLoginStatusRequest { session_id }),
    )
    .await
    .context("timed out reading Tailnet sign-in status")??
    .into_inner();
    Ok(decode_tailnet_status(response))
}

pub async fn cancel_tailnet_login(session_id: String) -> Result<()> {
    let mut client = BurrowClient::from_uds().await?;
    timeout(
        RPC_TIMEOUT,
        client
            .tailnet_client
            .login_cancel(TailnetLoginCancelRequest { session_id }),
    )
    .await
    .context("timed out cancelling Tailnet sign-in")??;
    Ok(())
}

async fn add_network(network_type: NetworkType, payload: Vec<u8>) -> Result<i32> {
    let id = next_network_id().await?;
    let mut client = BurrowClient::from_uds().await?;
    timeout(
        RPC_TIMEOUT,
        client.networks_client.network_add(Network {
            id,
            r#type: network_type.into(),
            payload,
        }),
    )
    .await
    .context("timed out saving network to Burrow daemon")??;
    Ok(id)
}

async fn next_network_id() -> Result<i32> {
    let networks = list_networks().await?;
    Ok(networks.iter().map(|network| network.id).max().unwrap_or(0) + 1)
}

fn summarize_network(network: &Network) -> NetworkSummary {
    match network.r#type() {
        NetworkType::WireGuard => summarize_wireguard(network),
        NetworkType::Tailnet => summarize_tailnet(network),
    }
}

fn summarize_wireguard(network: &Network) -> NetworkSummary {
    let payload = String::from_utf8_lossy(&network.payload);
    let detail = payload
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('['))
        .unwrap_or("Stored WireGuard configuration")
        .to_owned();
    NetworkSummary {
        id: network.id,
        title: format!("WireGuard {}", network.id),
        detail,
    }
}

fn summarize_tailnet(network: &Network) -> NetworkSummary {
    match TailnetConfig::from_slice(&network.payload) {
        Ok(config) => {
            let title = config
                .tailnet
                .clone()
                .or(config.hostname.clone())
                .unwrap_or_else(|| "Tailnet".to_owned());
            let authority = config
                .authority
                .unwrap_or_else(|| "default authority".to_owned());
            let account = config.account.unwrap_or_else(|| "default".to_owned());
            NetworkSummary {
                id: network.id,
                title,
                detail: format!("{authority} - account {account}"),
            }
        }
        Err(error) => NetworkSummary {
            id: network.id,
            title: "Tailnet".to_owned(),
            detail: format!("Unable to read Tailnet payload: {error}"),
        },
    }
}

fn decode_tailnet_status(
    response: burrow::grpc_defs::TailnetLoginStatusResponse,
) -> TailnetLoginStatus {
    TailnetLoginStatus {
        session_id: response.session_id,
        backend_state: response.backend_state,
        auth_url: optional(response.auth_url),
        running: response.running,
        needs_login: response.needs_login,
        tailnet_name: optional(response.tailnet_name),
        self_dns_name: optional(response.self_dns_name),
        tailnet_ips: response.tailnet_ips,
        health: response.health,
    }
}

fn optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

pub fn normalized(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub fn normalized_optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

pub fn require_value(value: &str, label: &str) -> Result<String> {
    normalized_optional(value).ok_or_else(|| anyhow!("{label} is required"))
}
