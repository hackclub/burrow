use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tun::TunOptions;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum DaemonCommand {
    Start(DaemonStartOptions),
    ServerInfo,
    ServerConfig,
    Stop,
    ReloadConfig(String)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct DaemonStartOptions {
    pub tun: TunOptions,
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
