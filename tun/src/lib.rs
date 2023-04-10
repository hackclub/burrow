#[cfg(target_vendor = "apple")]
#[path = "apple/mod.rs"]
mod imp;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod imp;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub(crate) mod unix;

pub use imp::{TunInterface, TunQueue};
