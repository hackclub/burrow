use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tun::TunOptions;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "method", content = "params")]
pub enum DaemonCommand {
    Start(DaemonStartOptions),
    ServerInfo,
    ServerConfig,
    Stop,
    ReloadConfig(String),
    AddConfig(AddConfigOptions),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct DaemonStartOptions {
    pub tun: TunOptions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct AddConfigOptions {
    pub content: String,
    pub fmt: String,
    pub interface_id: Option<i64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DaemonRequest {
    pub id: u64,
    #[serde(flatten)]
    pub command: DaemonCommand,
}

#[test]
fn test_daemoncommand_serialization() {
    insta::assert_snapshot!(serde_json::to_string(&DaemonCommand::Start(
        DaemonStartOptions::default()
    ))
    .unwrap());
    insta::assert_snapshot!(
        serde_json::to_string(&DaemonCommand::Start(DaemonStartOptions {
            tun: TunOptions { ..TunOptions::default() }
        }))
        .unwrap()
    );
    insta::assert_snapshot!(serde_json::to_string(&DaemonCommand::ServerInfo).unwrap());
    insta::assert_snapshot!(serde_json::to_string(&DaemonCommand::Stop).unwrap());
    insta::assert_snapshot!(serde_json::to_string(&DaemonCommand::ServerConfig).unwrap())
}
