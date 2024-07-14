#[cfg(target_family = "unix")]
mod unix;

#[cfg(target_family = "unix")]
pub use unix::{get_socket_path, DaemonClient, Listener};

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::{DaemonClient, Listener};
