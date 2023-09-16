#![deny(missing_debug_implementations)]

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod os_imp;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[path = "unix/mod.rs"]
pub(crate) mod os_imp;

mod options;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[cfg(feature = "tokio")]
pub mod tokio;

pub use options::TunOptions;
pub use os_imp::{TunInterface, TunQueue};
