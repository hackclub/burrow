#[cfg(feature = "tokio")]
#[path = "tokio/mod.rs"]
pub(crate) mod imp;

pub use imp::TunQueue;
