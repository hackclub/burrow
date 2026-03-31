use std::{net::SocketAddr, path::PathBuf, str};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub identity: Option<String>,
    #[serde(default)]
    pub address: Vec<String>,
    #[serde(default)]
    pub dns: Vec<String>,
    #[serde(default)]
    pub mtu: Option<u32>,
    #[serde(default)]
    pub tun_name: Option<String>,
    #[serde(default)]
    pub arti: ArtiConfig,
    #[serde(default)]
    pub tcp_stack: TcpStackConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtiConfig {
    pub state_dir: String,
    pub cache_dir: String,
}

impl Default for ArtiConfig {
    fn default() -> Self {
        Self {
            state_dir: "/var/lib/burrow/arti/state".to_string(),
            cache_dir: "/var/cache/burrow/arti".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TcpStackConfig {
    System(SystemTcpStackConfig),
}

impl Default for TcpStackConfig {
    fn default() -> Self {
        Self::System(SystemTcpStackConfig::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemTcpStackConfig {
    #[serde(default = "default_system_listen")]
    pub listen: String,
}

impl Default for SystemTcpStackConfig {
    fn default() -> Self {
        Self {
            listen: default_system_listen(),
        }
    }
}

impl Config {
    pub fn from_payload(payload: &[u8]) -> Result<Self> {
        if let Ok(config) = serde_json::from_slice(payload) {
            return Ok(config);
        }

        let payload = str::from_utf8(payload).context("tor payload must be valid UTF-8")?;
        toml::from_str(payload).context("failed to parse tor payload as JSON or TOML")
    }

    pub fn listen_addr(&self) -> Result<SocketAddr> {
        match &self.tcp_stack {
            TcpStackConfig::System(config) => config
                .listen
                .parse()
                .with_context(|| format!("invalid system tcp listen address '{}'", config.listen)),
        }
    }

    pub fn authority(&self) -> String {
        "arti://local".to_owned()
    }

    pub fn account_name(&self) -> String {
        self.account
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "default".to_owned())
    }

    pub fn identity_name(&self, network_id: i32) -> String {
        self.identity
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| self.tun_name.clone())
            .unwrap_or_else(|| format!("tor-{network_id}"))
    }

    pub fn runtime_dirs(&self, network_id: i32) -> (String, String) {
        let authority = sanitize_path_component(&self.authority());
        let account = sanitize_path_component(&self.account_name());
        let identity = sanitize_path_component(&self.identity_name(network_id));
        (
            append_runtime_path(&self.arti.state_dir, &[&authority, &account, &identity]),
            append_runtime_path(&self.arti.cache_dir, &[&authority, &account, &identity]),
        )
    }
}

fn default_system_listen() -> String {
    "127.0.0.1:9040".to_string()
}

fn append_runtime_path(base: &str, parts: &[&str]) -> String {
    let mut path = PathBuf::from(base);
    for part in parts {
        path.push(part);
    }
    path.to_string_lossy().to_string()
}

fn sanitize_path_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "default".to_owned()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_payload() {
        let payload = br#"{
            "address":["100.64.0.2/32"],
            "mtu":1400,
            "arti":{"state_dir":"/tmp/state","cache_dir":"/tmp/cache"},
            "tcp_stack":{"kind":"system","listen":"127.0.0.1:9150"}
        }"#;

        let config = Config::from_payload(payload).unwrap();
        assert_eq!(config.address, vec!["100.64.0.2/32"]);
        assert_eq!(config.listen_addr().unwrap().to_string(), "127.0.0.1:9150");
        assert!(config.runtime_dirs(7).0.contains("arti___local"));
    }

    #[test]
    fn parses_toml_payload() {
        let payload = r#"
address = ["100.64.0.3/32"]
mtu = 1280
tun_name = "burrow-tor"

[arti]
state_dir = "/tmp/state"
cache_dir = "/tmp/cache"

[tcp_stack]
kind = "system"
listen = "127.0.0.1:9140"
"#;

        let config = Config::from_payload(payload.as_bytes()).unwrap();
        assert_eq!(config.tun_name.as_deref(), Some("burrow-tor"));
        assert_eq!(config.listen_addr().unwrap().to_string(), "127.0.0.1:9140");
        assert_eq!(config.identity_name(11), "burrow-tor");
    }
}
