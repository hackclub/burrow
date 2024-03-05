#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub mod wireguard;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod daemon;
pub(crate) mod tracing;
pub mod database;

#[cfg(target_vendor = "apple")]
pub use daemon::apple::spawn_in_process;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub use daemon::{
    DaemonClient,
    DaemonCommand,
    DaemonResponse,
    DaemonResponseData,
    DaemonStartOptions,
    ServerInfo,
};
