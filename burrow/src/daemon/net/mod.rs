use serde::{Deserialize, Serialize};

use super::DaemonCommand;

#[cfg(target_family = "unix")]
mod unix;

#[cfg(target_family = "unix")]
pub use unix::{DaemonClient, Listener};

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::{DaemonClient, Listener};

#[derive(Clone, Serialize, Deserialize)]
pub struct DaemonRequest {
    pub id: u64,
    pub command: DaemonCommand,
}
