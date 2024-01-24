#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub mod wireguard;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
mod daemon;
#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub use daemon::{
    DaemonClient,
    DaemonCommand,
    DaemonResponse,
    DaemonResponseData,
    DaemonStartOptions,
    ServerInfo,
};

#[cfg(target_vendor = "apple")]
mod apple;

#[cfg(target_vendor = "apple")]
pub use apple::*;
