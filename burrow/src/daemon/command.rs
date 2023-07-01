use serde::{Deserialize, Serialize};
use tun::TunOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonCommand {
    Start(DaemonStartOptions),
    Stop,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DaemonStartOptions {
    pub(super) tun: TunOptions,
}
