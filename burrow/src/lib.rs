pub mod ensureroot;

use std::{
    mem,
    os::fd::{AsRawFd, FromRawFd},
};

use tun::TunInterface;

#[no_mangle]
pub extern "C" fn start() -> i32 {
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
        .next()
        .unwrap();

    iface2.as_raw_fd()
}
