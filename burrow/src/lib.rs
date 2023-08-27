pub mod ensureroot;
#[cfg(target_vendor = "apple")]
mod apple;
mod server;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use std::{
    mem,
    os::fd::{AsRawFd, FromRawFd},
};
use std::os::fd::RawFd;
use tracing::debug;

use tun::TunInterface;

#[cfg(target_vendor = "apple")]
pub use apple::{NetWorkSettings, getNetworkSettings, initialize_oslog};

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
pub use server::spawn_server;
// TODO Separate start and retrieve functions

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[no_mangle]
pub extern "C" fn retrieve() -> i32 {
    let iface2 = (1..100)
        .filter_map(|i| {
            debug!("Getting TunInterface with fd: {:?}", i);
            let iface = unsafe { TunInterface::from_raw_fd(i) };
            match iface.name() {
                Ok(name) => {
                    debug!("Found interface {}", name);
                    Some(iface)
                },
                Err(_) => {
                    mem::forget(iface);
                    None
                }
            }
        })
        .next();
    match iface2 {
        Some(iface) => {
            debug!("Found interface {:?}", iface.name());
            iface.as_raw_fd()
        },
        None => {
            debug!("No interface found");
            -1
        }
    }
}

pub fn get_iface() -> Option<TunInterface> {
    (1..100)
        .filter_map(|i| {
            debug!("Getting TunInterface with fd: {:?}", i);
            let iface = unsafe { TunInterface::from_raw_fd(i) };
            match iface.name() {
                Ok(name) => {
                    debug!("Found interface {}", name);
                    Some(iface)
                },
                Err(_) => {
                    mem::forget(iface);
                    None
                }
            }
        })
        .next()
}
