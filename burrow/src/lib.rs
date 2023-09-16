pub mod ensureroot;
pub mod wireguard;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
use std::{
    mem,
    os::fd::{AsRawFd, FromRawFd},
};

use tun::TunInterface;

// TODO Separate start and retrieve functions

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[no_mangle]
pub extern "C" fn retrieve() -> i32 {
    let iface2 = (1..100)
        .filter_map(|i| {
            let iface = unsafe { TunInterface::from_raw_fd(i) };
            match iface.name() {
                Ok(_name) => Some(iface),
                Err(_) => {
                    mem::forget(iface);
                    None
                }
            }
        })
        .next();
    match iface2 {
        Some(iface) => iface.as_raw_fd(),
        None => -1,
    }
}
