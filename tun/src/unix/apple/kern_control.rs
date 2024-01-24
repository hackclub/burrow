use std::{io::Error, mem::size_of, os::unix::io::AsRawFd};

use fehler::throws;

use super::sys;

/// Trait to connect to kernel extensions on Apple platforms
///
/// Pulled from XNU source: https://github.com/apple/darwin-xnu/blob/main/bsd/sys/kern_control.h
pub trait SysControlSocket {
    #[throws]
    fn resolve(&self, name: &str, index: u32) -> socket2::SockAddr;
}

impl SysControlSocket for socket2::Socket {
    #[throws]
    fn resolve(&self, name: &str, index: u32) -> socket2::SockAddr {
        let mut info = sys::ctl_info { ctl_id: 0, ctl_name: [0; 96] };
        info.ctl_name[..name.len()].copy_from_slice(name.as_bytes());

        unsafe { sys::resolve_ctl_info(self.as_raw_fd(), &mut info as *mut sys::ctl_info)? };

        let (_, addr) = unsafe {
            socket2::SockAddr::init(|addr_storage, len| {
                *len = size_of::<sys::sockaddr_ctl>() as u32;

                let addr: &mut sys::sockaddr_ctl = &mut *addr_storage.cast();
                addr.sc_len = *len as u8;
                addr.sc_family = sys::AF_SYSTEM as u8;
                addr.ss_sysaddr = sys::AF_SYS_CONTROL as u16;
                addr.sc_id = info.ctl_id;
                addr.sc_unit = index;
                Ok(())
            })
        }?;

        addr
    }
}
