use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tun::TunOptions;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag="method", content="params")]
pub enum DaemonCommand {
    Start(DaemonStartOptions),
    ServerInfo,
    ServerConfig,
    Stop,
    ReloadConfig(String),
    AddConfigToml(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct DaemonStartOptions {
    pub tun: TunOptions,
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
