#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub mod wireguard;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod daemon;
pub mod database;
pub(crate) mod tracing;

#[cfg(target_vendor = "apple")]
pub use daemon::apple::spawn_in_process;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub use daemon::{
    rpc::DaemonResponse,
    rpc::ServerInfo,
    DaemonClient,
    DaemonCommand,
    DaemonResponseData,
    DaemonStartOptions,
};
