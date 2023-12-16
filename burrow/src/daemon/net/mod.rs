use serde::{Deserialize, Serialize};

use super::DaemonCommand;

#[cfg(target_family = "unix")]
mod unix;
#[cfg(all(target_family = "unix", not(target_os = "linux")))]
pub use unix::{listen, DaemonClient};

#[cfg(target_os = "linux")]
mod systemd;
#[cfg(target_os = "linux")]
pub use systemd::{listen, DaemonClient};

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::{listen, DaemonClient};

#[cfg(target_vendor = "apple")]
mod apple;

#[cfg(target_vendor = "apple")]
pub use apple::start_srv;

#[derive(Clone, Serialize, Deserialize)]
pub struct DaemonRequest {
    pub id: u32,
    pub command: DaemonCommand,
}
