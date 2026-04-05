use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tun::TunInterface;

use crate::wireguard::Config;

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
pub struct DaemonResponse {
    //  Error types can't be serialized, so this is the second best option.
    pub result: Result<DaemonResponseData, String>,
    pub id: u64,
}

impl DaemonResponse {
    pub fn new(result: Result<DaemonResponseData, impl ToString>) -> Self {
        Self {
            result: result.map_err(|e| e.to_string()),
            id: 0,
        }
    }
}

impl From<DaemonResponseData> for DaemonResponse {
    fn from(val: DaemonResponseData) -> Self {
        DaemonResponse::new(Ok::<DaemonResponseData, String>(val))
    }
}

impl DaemonResponse {
    pub fn with_id(self, id: u64) -> Self {
        Self { id, ..self }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServerInfo {
    pub name: Option<String>,
    pub ip: Option<String>,
    #[serde(default)]
    pub ipv6: Vec<String>,
    pub mtu: Option<i32>,
}

impl TryFrom<&TunInterface> for ServerInfo {
    type Error = anyhow::Error;

    #[cfg(any(target_os = "linux", target_vendor = "apple"))]
    fn try_from(server: &TunInterface) -> anyhow::Result<Self> {
        Ok(ServerInfo {
            name: server.name().ok(),
            ip: server.ipv4_addr().ok().map(|ip| ip.to_string()),
            ipv6: server
                .ipv6_addrs()
                .unwrap_or_default()
                .into_iter()
                .map(|ip| ip.to_string())
                .collect(),
            mtu: server.mtu().ok(),
        })
    }

    #[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
    fn try_from(server: &TunInterface) -> anyhow::Result<Self> {
        Err(anyhow!("Not implemented in this platform"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServerConfig {
    pub address: Vec<String>,
    #[serde(default)]
    pub routes: Vec<String>,
    #[serde(default)]
    pub dns_servers: Vec<String>,
    #[serde(default)]
    pub search_domains: Vec<String>,
    #[serde(default)]
    pub include_default_route: bool,
    pub name: Option<String>,
    pub mtu: Option<i32>,
}

impl TryFrom<&Config> for ServerConfig {
    type Error = anyhow::Error;

    fn try_from(config: &Config) -> anyhow::Result<Self> {
        Ok(ServerConfig {
            address: config.interface.address.clone(),
            routes: config
                .peers
                .iter()
                .flat_map(|peer| peer.allowed_ips.iter().cloned())
                .collect(),
            dns_servers: config.interface.dns.clone(),
            search_domains: Vec::new(),
            include_default_route: false,
            name: None,
            mtu: config.interface.mtu.map(|mtu| mtu as i32),
        })
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: vec!["10.13.13.2".to_string()], // Dummy remote address
            routes: Vec::new(),
            dns_servers: Vec::new(),
            search_domains: Vec::new(),
            include_default_route: false,
            name: None,
            mtu: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum DaemonResponseData {
    ServerInfo(ServerInfo),
    ServerConfig(ServerConfig),
    None,
}

#[test]
fn test_response_serialization() -> anyhow::Result<()> {
    insta::assert_snapshot!(serde_json::to_string(&DaemonResponse::new(Ok::<
        DaemonResponseData,
        String,
    >(
        DaemonResponseData::None
    )))?);
    insta::assert_snapshot!(serde_json::to_string(&DaemonResponse::new(Ok::<
        DaemonResponseData,
        String,
    >(
        DaemonResponseData::ServerInfo(ServerInfo {
            name: Some("burrow".to_string()),
            ip: None,
            ipv6: Vec::new(),
            mtu: Some(1500)
        })
    )))?);
    insta::assert_snapshot!(serde_json::to_string(&DaemonResponse::new(Err::<
        DaemonResponseData,
        String,
    >(
        "error".to_string()
    )))?);
    insta::assert_snapshot!(serde_json::to_string(&DaemonResponse::new(Ok::<
        DaemonResponseData,
        String,
    >(
        DaemonResponseData::ServerConfig(ServerConfig::default())
    )))?);
    Ok(())
}
