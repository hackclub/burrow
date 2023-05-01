#[cfg(target_vendor = "apple")]
#[path = "apple/mod.rs"]
mod imp;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod imp;

pub use imp::reroute;

pub fn ensure_root() {
    todo!();
}
