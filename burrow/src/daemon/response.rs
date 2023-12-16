use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tun::TunInterface;

#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
pub struct DaemonResponse {
    //  Error types can't be serialized, so this is the second best option.
    pub result: Result<DaemonResponseData, String>,
    pub id: u32,
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
    pub fn with_id(self, id: u32) -> Self {
        Self { id, ..self }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct ServerInfo {
    pub name: Option<String>,
    pub ip: Option<String>,
    pub mtu: Option<i32>,
}

impl TryFrom<&TunInterface> for ServerInfo {
    type Error = anyhow::Error;

    #[cfg(any(target_os = "linux", target_vendor = "apple"))]
    fn try_from(server: &TunInterface) -> anyhow::Result<Self> {
        Ok(ServerInfo {
            name: server.name().ok(),
            ip: server.ipv4_addr().ok().map(|ip| ip.to_string()),
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
    pub address: Option<String>,
    pub name: Option<String>,
    pub mtu: Option<i32>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: Some("10.13.13.2".to_string()), // Dummy remote address
            name: None,
            mtu: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
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
