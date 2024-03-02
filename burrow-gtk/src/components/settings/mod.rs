use super::*;
use diag::{StatusTernary, SystemSetup};

mod daemon_group;
mod diag_group;

pub use daemon_group::{DaemonGroup, DaemonGroupInit, DaemonGroupMsg};
pub use diag_group::{DiagGroup, DiagGroupInit, DiagGroupMsg};
