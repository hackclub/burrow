use rpc::ServerConfig;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::daemon::rpc;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "method", content = "params")]
pub enum DaemonNotification {
    ConfigChange(ServerConfig),
}
