#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod imp;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[path = "unix/mod.rs"]
pub(crate) mod imp;

pub use imp::{ensure_root, reroute};

pub fn log<T: std::fmt::Display>(t: T) {
    #[cfg(debug_assertions)]
    {
        println!("[Burrow] {t}");
    }
}
