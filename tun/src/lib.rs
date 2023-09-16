#![deny(missing_debug_implementations)]

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod imp;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[path = "unix/mod.rs"]
pub(crate) mod imp;

mod options;
pub mod routing;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[cfg(feature = "tokio")]
pub mod tokio;

pub use imp::{TunInterface, TunQueue};
pub use options::TunOptions;
