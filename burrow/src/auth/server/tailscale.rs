use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
    task::JoinHandle,
};

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TailscaleLoginStartRequest {
    pub account_name: String,
    pub identity_name: String,
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub control_url: Option<String>,
    #[serde(default)]
    pub packet_socket: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TailscaleLoginStatus {
    pub backend_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_url: Option<String>,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub needs_login: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tailnet_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magic_dns_suffix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_dns_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tailscale_ips: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub health: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TailscaleLoginStartResponse {
    pub session_id: String,
    pub status: TailscaleLoginStatus,
}

pub struct TailscaleLoginSession {
    pub session_id: String,
    pub helper: Arc<TailscaleHelperProcess>,
    pub status: TailscaleLoginStatus,
}

#[derive(Clone, Default)]
pub struct TailscaleBridgeManager {
    client: Client,
    sessions: Arc<Mutex<HashMap<String, Arc<ManagedSession>>>>,
}

pub struct TailscaleHelperProcess {
    session_id: String,
    listen_url: String,
    packet_socket: Option<PathBuf>,
    control_url: Option<String>,
    state_dir: PathBuf,
    child: Arc<Mutex<Child>>,
    _stderr_task: JoinHandle<()>,
}

type ManagedSession = TailscaleHelperProcess;

#[derive(Debug, Deserialize)]
struct HelperHello {
    listen_addr: String,
    #[serde(default)]
    packet_socket: Option<String>,
}

impl TailscaleBridgeManager {
    pub async fn start_login(
        &self,
        request: TailscaleLoginStartRequest,
    ) -> Result<TailscaleLoginStartResponse> {
        let session = self.ensure_session(request).await?;
        Ok(TailscaleLoginStartResponse {
            session_id: session.session_id,
            status: session.status,
        })
    }

    pub async fn ensure_session(
        &self,
        request: TailscaleLoginStartRequest,
    ) -> Result<TailscaleLoginSession> {
        let key = session_key_for_request(&request);
        let requested_packet_socket = request
            .packet_socket
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let requested_control_url = request
            .control_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        if let Some(existing) = self.sessions.lock().await.get(&key).cloned() {
            let needs_restart_for_socket = match (requested_packet_socket, existing.packet_socket())
            {
                (Some(requested), Some(current)) => current != Path::new(requested),
                (Some(_), None) => true,
                _ => false,
            };
            let needs_restart_for_control_url =
                requested_control_url != existing.control_url().map(|value| value.trim());

            if !needs_restart_for_socket && !needs_restart_for_control_url {
                match self.fetch_status(existing.as_ref()).await {
                    Ok(status) => {
                        return Ok(TailscaleLoginSession {
                            session_id: existing.session_id.clone(),
                            helper: existing,
                            status,
                        });
                    }
                    Err(err) => {
                        log::warn!(
                            "tailscale login session {} is stale, restarting: {err}",
                            existing.session_id
                        );
                    }
                }
            } else {
                log::info!(
                    "tailscale login session {} no longer matches requested transport, restarting",
                    existing.session_id
                );
            }

            self.sessions.lock().await.remove(&key);
            let _ = self.shutdown_session(existing.as_ref()).await;
        }

        let session = Arc::new(spawn_tailscale_helper(&request).await?);
        let status = self.wait_for_status(session.as_ref()).await?;
        let response = TailscaleLoginSession {
            session_id: session.session_id.clone(),
            helper: session.clone(),
            status,
        };

        self.sessions.lock().await.insert(key, session);
        Ok(response)
    }

    pub async fn status(&self, session_id: &str) -> Result<Option<TailscaleLoginStatus>> {
        let session = {
            let sessions = self.sessions.lock().await;
            sessions
                .values()
                .find(|session| session.session_id == session_id)
                .cloned()
        };

        match session {
            Some(session) => match self.fetch_status(session.as_ref()).await {
                Ok(status) => Ok(Some(status)),
                Err(err) => {
                    self.remove_session_by_id(session_id).await;
                    Err(err)
                }
            },
            None => Ok(None),
        }
    }

    pub async fn cancel(&self, session_id: &str) -> Result<bool> {
        let session = self.remove_session_by_id(session_id).await;
        match session {
            Some(session) => {
                self.shutdown_session(session.as_ref()).await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn wait_for_status(&self, session: &ManagedSession) -> Result<TailscaleLoginStatus> {
        let mut last_error = None;
        let mut last_status = None;
        for _ in 0..40 {
            match session.status_with_client(&self.client).await {
                Ok(status) if status.running || status.auth_url.is_some() => return Ok(status),
                Ok(status) => last_status = Some(status),
                Err(err) => last_error = Some(err),
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        if let Some(status) = last_status {
            return Ok(status);
        }
        Err(last_error.unwrap_or_else(|| anyhow!("tailscale helper did not become ready")))
    }

    async fn fetch_status(&self, session: &ManagedSession) -> Result<TailscaleLoginStatus> {
        session.status_with_client(&self.client).await
    }

    async fn remove_session_by_id(&self, session_id: &str) -> Option<Arc<ManagedSession>> {
        let mut sessions = self.sessions.lock().await;
        let key = sessions
            .iter()
            .find_map(|(key, session)| (session.session_id == session_id).then(|| key.clone()))?;
        sessions.remove(&key)
    }

    async fn shutdown_session(&self, session: &ManagedSession) -> Result<()> {
        session.shutdown_with_client(&self.client).await
    }
}

impl TailscaleHelperProcess {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn packet_socket(&self) -> Option<&Path> {
        self.packet_socket.as_deref()
    }

    pub fn control_url(&self) -> Option<&str> {
        self.control_url.as_deref()
    }

    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    pub async fn status(&self) -> Result<TailscaleLoginStatus> {
        self.status_with_client(&Client::new()).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.shutdown_with_client(&Client::new()).await
    }

    async fn status_with_client(&self, client: &Client) -> Result<TailscaleLoginStatus> {
        let mut child = self.child.lock().await;
        if let Some(status) = child.try_wait()? {
            return Err(anyhow!(
                "tailscale helper exited with status {status} for {}",
                self.state_dir.display()
            ));
        }
        drop(child);

        let response = client
            .get(format!("{}/status", self.listen_url))
            .send()
            .await
            .context("failed to query tailscale helper status")?
            .error_for_status()
            .context("tailscale helper status request failed")?;

        let status = response
            .json::<TailscaleLoginStatus>()
            .await
            .context("invalid tailscale helper status response")?;

        log::info!(
            "tailscale helper status session={} backend_state={} running={} needs_login={} auth_url={:?}",
            self.session_id,
            status.backend_state,
            status.running,
            status.needs_login,
            status.auth_url
        );
        Ok(status)
    }

    async fn shutdown_with_client(&self, client: &Client) -> Result<()> {
        let _ = client.post(format!("{}/shutdown", self.listen_url)).send().await;

        for _ in 0..10 {
            let mut child = self.child.lock().await;
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            drop(child);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let mut child = self.child.lock().await;
        child
            .start_kill()
            .context("failed to kill tailscale helper")?;
        let _ = child.wait().await;
        Ok(())
    }
}

pub async fn spawn_tailscale_helper(
    request: &TailscaleLoginStartRequest,
) -> Result<TailscaleHelperProcess> {
    let state_dir = state_root().join(session_dir_name(request));
    tokio::fs::create_dir_all(&state_dir)
        .await
        .with_context(|| format!("failed to create {}", state_dir.display()))?;

    let mut child = helper_command(request, &state_dir)?
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn tailscale login helper")?;

    let stdout = child
        .stdout
        .take()
        .context("tailscale helper stdout unavailable")?;
    let stderr = child
        .stderr
        .take()
        .context("tailscale helper stderr unavailable")?;

    let hello_line = tokio::time::timeout(Duration::from_secs(20), async move {
        let mut lines = BufReader::new(stdout).lines();
        lines.next_line().await
    })
    .await
    .context("timed out waiting for tailscale helper startup")??
    .context("tailscale helper exited before reporting listen address")?;

    let hello: HelperHello =
        serde_json::from_str(&hello_line).context("invalid tailscale helper startup line")?;

    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            log::info!("tailscale-login-bridge: {line}");
        }
    });

    Ok(TailscaleHelperProcess {
        session_id: random_session_id(),
        listen_url: format!("http://{}", hello.listen_addr),
        packet_socket: hello.packet_socket.map(PathBuf::from),
        control_url: request.control_url.clone(),
        state_dir,
        child: Arc::new(Mutex::new(child)),
        _stderr_task: stderr_task,
    })
}

fn helper_command(request: &TailscaleLoginStartRequest, state_dir: &Path) -> Result<Command> {
    let mut command = if let Ok(path) = env::var("BURROW_TAILSCALE_HELPER") {
        Command::new(path)
    } else {
        let helper_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("Tools/tailscale-login-bridge");
        let mut command = Command::new("go");
        command.current_dir(helper_dir).arg("run").arg(".");
        command.env("GOWORK", "off");
        command
    };

    command
        .arg("--listen")
        .arg("127.0.0.1:0")
        .arg("--state-dir")
        .arg(state_dir)
        .arg("--hostname")
        .arg(default_hostname(request));

    if let Some(control_url) = request.control_url.as_deref() {
        let trimmed = control_url.trim();
        if !trimmed.is_empty() {
            command.arg("--control-url").arg(trimmed);
        }
    }

    if let Some(packet_socket) = request.packet_socket.as_deref() {
        let trimmed = packet_socket.trim();
        if !trimmed.is_empty() {
            command.arg("--packet-socket").arg(trimmed);
        }
    }

    Ok(command)
}

pub(crate) fn packet_socket_path(request: &TailscaleLoginStartRequest) -> PathBuf {
    state_root().join(session_dir_name(request)).join("packet.sock")
}

pub(crate) fn state_root() -> PathBuf {
    if let Ok(path) = env::var("BURROW_TAILSCALE_STATE_ROOT") {
        return PathBuf::from(path);
    }

    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if cfg!(target_vendor = "apple") {
        return home
            .join("Library")
            .join("Application Support")
            .join("Burrow")
            .join("tailscale");
    }
    home.join(".local")
        .join("share")
        .join("burrow")
        .join("tailscale")
}

pub(crate) fn session_dir_name(request: &TailscaleLoginStartRequest) -> String {
    format!(
        "{}-{}-{}",
        slug(&request.account_name),
        slug(&request.identity_name),
        slug(control_scope(request))
    )
}

fn session_key_for_request(request: &TailscaleLoginStartRequest) -> String {
    format!(
        "{}:{}:{}",
        request.account_name,
        request.identity_name,
        control_scope(request)
    )
}

fn control_scope(request: &TailscaleLoginStartRequest) -> &str {
    request
        .control_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("tailscale-managed")
}

pub(crate) fn default_hostname(request: &TailscaleLoginStartRequest) -> String {
    request
        .hostname
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("burrow-{}", slug(&request.identity_name)))
}

fn random_session_id() -> String {
    let mut bytes = [0_u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn slug(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            output.push('-');
        }
    }
    if output.is_empty() {
        "default".to_owned()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_sanitizes_input() {
        assert_eq!(slug("Apple Phone"), "applephone");
        assert_eq!(slug("default_identity"), "default-identity");
        assert_eq!(slug(""), "default");
    }

    #[test]
    fn state_dir_is_scoped_by_account_identity_and_control_plane() {
        let request = TailscaleLoginStartRequest {
            account_name: "default".to_owned(),
            identity_name: "apple".to_owned(),
            hostname: None,
            control_url: None,
            packet_socket: None,
        };
        assert_eq!(session_dir_name(&request), "default-apple-tailscale-managed");
        assert_eq!(default_hostname(&request), "burrow-apple");

        let custom_request = TailscaleLoginStartRequest {
            control_url: Some("https://ts.burrow.net".to_owned()),
            ..request
        };
        assert_eq!(
            session_dir_name(&custom_request),
            "default-apple-httpstsburrownet"
        );
    }
}
