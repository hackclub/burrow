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

#[derive(Clone, Default)]
pub struct TailscaleBridgeManager {
    client: Client,
    sessions: Arc<Mutex<HashMap<String, Arc<ManagedSession>>>>,
}

struct ManagedSession {
    session_id: String,
    listen_url: String,
    state_dir: PathBuf,
    child: Arc<Mutex<Child>>,
    _stderr_task: JoinHandle<()>,
}

#[derive(Debug, Deserialize)]
struct HelperHello {
    listen_addr: String,
}

impl TailscaleBridgeManager {
    pub async fn start_login(
        &self,
        request: TailscaleLoginStartRequest,
    ) -> Result<TailscaleLoginStartResponse> {
        let key = session_key(&request.account_name, &request.identity_name);

        if let Some(existing) = self.sessions.lock().await.get(&key).cloned() {
            match self.fetch_status(existing.as_ref()).await {
                Ok(status) => {
                    return Ok(TailscaleLoginStartResponse {
                        session_id: existing.session_id.clone(),
                        status,
                    });
                }
                Err(err) => {
                    log::warn!(
                        "tailscale login session {} is stale, restarting: {err}",
                        existing.session_id
                    );
                    self.sessions.lock().await.remove(&key);
                    let _ = self.shutdown_session(existing.as_ref()).await;
                }
            }
        }

        let state_dir = state_root().join(session_dir_name(&request));
        tokio::fs::create_dir_all(&state_dir)
            .await
            .with_context(|| format!("failed to create {}", state_dir.display()))?;

        let mut child = helper_command(&request, &state_dir)?
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

        let session = Arc::new(ManagedSession {
            session_id: random_session_id(),
            listen_url: format!("http://{}", hello.listen_addr),
            state_dir,
            child: Arc::new(Mutex::new(child)),
            _stderr_task: stderr_task,
        });

        let status = self.wait_for_status(session.as_ref()).await?;
        let response = TailscaleLoginStartResponse {
            session_id: session.session_id.clone(),
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
            match self.fetch_status(session).await {
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
        let mut child = session.child.lock().await;
        if let Some(status) = child.try_wait()? {
            return Err(anyhow!(
                "tailscale helper exited with status {status} for {}",
                session.state_dir.display()
            ));
        }
        drop(child);

        let response = self
            .client
            .get(format!("{}/status", session.listen_url))
            .send()
            .await
            .context("failed to query tailscale helper status")?
            .error_for_status()
            .context("tailscale helper status request failed")?;

        response
            .json::<TailscaleLoginStatus>()
            .await
            .context("invalid tailscale helper status response")
    }

    async fn remove_session_by_id(&self, session_id: &str) -> Option<Arc<ManagedSession>> {
        let mut sessions = self.sessions.lock().await;
        let key = sessions
            .iter()
            .find_map(|(key, session)| (session.session_id == session_id).then(|| key.clone()))?;
        sessions.remove(&key)
    }

    async fn shutdown_session(&self, session: &ManagedSession) -> Result<()> {
        let _ = self
            .client
            .post(format!("{}/shutdown", session.listen_url))
            .send()
            .await;

        for _ in 0..10 {
            let mut child = session.child.lock().await;
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            drop(child);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let mut child = session.child.lock().await;
        child
            .start_kill()
            .context("failed to kill tailscale helper")?;
        let _ = child.wait().await;
        Ok(())
    }
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

    Ok(command)
}

fn state_root() -> PathBuf {
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

fn session_dir_name(request: &TailscaleLoginStartRequest) -> String {
    format!(
        "{}-{}",
        slug(&request.account_name),
        slug(&request.identity_name)
    )
}

fn session_key(account_name: &str, identity_name: &str) -> String {
    format!("{account_name}:{identity_name}")
}

fn default_hostname(request: &TailscaleLoginStartRequest) -> String {
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
    fn state_dir_is_stable_by_account_and_identity() {
        let request = TailscaleLoginStartRequest {
            account_name: "default".to_owned(),
            identity_name: "apple".to_owned(),
            hostname: None,
            control_url: None,
        };
        assert_eq!(session_dir_name(&request), "default-apple");
        assert_eq!(default_hostname(&request), "burrow-apple");
    }
}
