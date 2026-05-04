#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub mod control;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub mod wireguard;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod auth;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod daemon;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub mod database;
#[cfg(target_os = "linux")]
pub mod tor;
pub(crate) mod tracing;
#[cfg(target_os = "linux")]
pub mod usernet;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub use daemon::apple::{spawn_in_process, spawn_in_process_with_paths};
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub use daemon::{
    rpc::grpc_defs, rpc::BurrowClient, rpc::DaemonResponse, rpc::ServerInfo, DaemonClient,
    DaemonCommand, DaemonResponseData, DaemonStartOptions,
};
