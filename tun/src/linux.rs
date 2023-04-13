use std::fs::OpenOptions;
use std::io::Result;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};

use crate::unix::copy_if_name;

pub struct TunInterface {
    inner: socket2::Socket,
}

impl TunInterface {
    pub fn new() -> Result<TunInterface> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")?;

        let iff = libc::ifreq {
            ifr_name: [0; libc::IFNAMSIZ],
            ifr_ifru: libc::__c_anonymous_ifr_ifru {
                ifru_flags: (libc::IFF_TUN | libc::IFF_TUN_EXCL | libc::IFF_NO_PI) as i16,
            },
        };
        unsafe { sys::tun_set_iff(file.as_raw_fd(), &iff)? };

        let inner = unsafe { socket2::Socket::from_raw_fd(file.into_raw_fd()) };
        Ok(TunInterface { inner })
    }

    pub fn name(&self) -> Result<String> {
        let mut iff = libc::ifreq {
            ifr_name: [0; libc::IFNAMSIZ],
            ifr_ifru: libc::__c_anonymous_ifr_ifru { ifru_flags: 0 },
        };
        unsafe { sys::tun_get_iff(self.inner.as_raw_fd(), &mut iff)? };

        let name = copy_if_name(iff.ifr_name);
        Ok(name)
    }
}

mod sys {
    use nix::{ioctl_read_bad, ioctl_write_ptr_bad, request_code_read, request_code_write};
    use std::mem::size_of;

    ioctl_write_ptr_bad!(
        tun_set_iff,
        request_code_write!(b'T', 202, size_of::<libc::c_int>()),
        libc::ifreq
    );
    ioctl_read_bad!(
        tun_get_iff,
        request_code_read!(b'T', 210, size_of::<libc::c_uint>()),
        libc::ifreq
    );
}

pub struct TunQueue;
