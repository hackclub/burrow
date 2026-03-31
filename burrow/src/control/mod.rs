pub mod config;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use config::{TailnetConfig, TailnetProvider};

pub const BURROW_CAPABILITY_VERSION: i32 = 1;
pub const BURROW_TAILNET_DOMAIN: &str = "burrow.net";

pub type NodeCapMap = BTreeMap<String, Vec<Value>>;
pub type PeerCapMap = BTreeMap<String, Vec<Value>>;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hostinfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub request_tags: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserProfile {
    pub id: i64,
    pub login_name: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_pic_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterAuth {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_access_token: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub id: i64,
    pub stable_id: String,
    pub name: String,
    pub user_id: i64,
    pub node_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disco_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_ips: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endpoints: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_derp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostinfo: Option<Hostinfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub primary_routes: Vec<String>,
    #[serde(default = "default_capability_version")]
    pub cap_version: i32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cap_map: NodeCapMap,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub peer_cap_map: PeerCapMap,
    #[serde(default)]
    pub machine_authorized: bool,
    #[serde(default)]
    pub node_key_expired: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub online: Option<bool>,
}

impl Node {
    pub fn preferred_name(request: &RegisterRequest) -> String {
        if let Some(name) = request.name.as_deref() {
            return name.to_owned();
        }
        if let Some(hostname) = request
            .hostinfo
            .as_ref()
            .and_then(|hostinfo| hostinfo.hostname.as_deref())
        {
            return hostname.to_owned();
        }
        format!("node-{}", short_key(&request.node_key))
    }

    pub fn normalized_allowed_ips(request: &RegisterRequest) -> Vec<String> {
        if request.allowed_ips.is_empty() {
            return request.addresses.clone();
        }
        request.allowed_ips.clone()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterRequest {
    #[serde(default = "default_capability_version")]
    pub version: i32,
    pub node_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_node_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disco_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<RegisterAuth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub followup: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostinfo: Option<Hostinfo>,
    #[serde(default)]
    pub ephemeral: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tailnet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_ips: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endpoints: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home_derp: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub primary_routes: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub cap_map: NodeCapMap,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub peer_cap_map: PeerCapMap,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct RegisterResponse {
    pub user: UserProfile,
    pub node: Node,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_url: Option<String>,
    pub machine_authorized: bool,
    pub node_key_expired: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MapRequest {
    #[serde(default = "default_capability_version")]
    pub version: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compress: Option<String>,
    #[serde(default)]
    pub keep_alive: bool,
    pub node_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disco_key: Option<String>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostinfo: Option<Hostinfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_session_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_session_seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endpoints: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub debug_flags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_handle: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DnsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolvers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub search_domains: Vec<String>,
    #[serde(default)]
    pub magic_dns: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacketFilter {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub destinations: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protocols: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MapResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_session_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<i64>,
    pub node: Node,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub peers: Vec<Node>,
    pub domain: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dns: Option<DnsConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packet_filters: Vec<PacketFilter>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalAuthRequest {
    pub identifier: String,
    pub password: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalAuthResponse {
    pub access_token: String,
    pub user: UserProfile,
}

fn default_capability_version() -> i32 {
    BURROW_CAPABILITY_VERSION
}

fn short_key(key: &str) -> String {
    key.chars().take(8).collect()
}
