use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tun::TunOptions;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum DaemonCommand {
    Start(DaemonStartOptions),
    ServerInfo,
    ServerConfig,
    Stop,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct DaemonStartOptions {
    pub(super) tun: TunOptions,
}

#[test]
fn test_daemoncommand_serialization() {
    insta::assert_snapshot!(
        serde_json::to_string(&DaemonCommand::Start(DaemonStartOptions::default())).unwrap()
    );
    insta::assert_snapshot!(
        serde_json::to_string(&DaemonCommand::ServerInfo).unwrap()
    );
    insta::assert_snapshot!(
        serde_json::to_string(&DaemonCommand::Stop).unwrap()
    );
    insta::assert_snapshot!(
        serde_json::to_string(&DaemonCommand::ServerConfig).unwrap()
    )
}