use std::{
    ffi::CStr,
    io::Error,
    mem,
    mem::MaybeUninit,
    net::{IpAddr, SocketAddr},
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
};

use tracing::instrument;

use crate::syscall;

mod queue;

#[cfg(target_vendor = "apple")]
#[path = "apple/mod.rs"]
mod imp;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod imp;

use fehler::throws;
pub use imp::TunInterface;
pub use queue::TunQueue;

impl AsRawFd for TunInterface {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl FromRawFd for TunInterface {
    unsafe fn from_raw_fd(fd: RawFd) -> TunInterface {
        let socket = socket2::Socket::from_raw_fd(fd);
        TunInterface { socket }
    }
}

impl IntoRawFd for TunInterface {
    fn into_raw_fd(self) -> RawFd {
        self.socket.into_raw_fd()
    }
}

unsafe fn assume_init(buf: &[MaybeUninit<u8>]) -> &[u8] {
    &*(buf as *const [MaybeUninit<u8>] as *const [u8])
}

impl TunInterface {
    #[throws]
    #[instrument]
    pub fn recv(&self, buf: &mut [u8]) -> usize {
        // Use IoVec to read directly into target buffer
        let mut tmp_buf = [MaybeUninit::uninit(); 1500];
        let len = self.socket.recv(&mut tmp_buf)?;
        let result_buf = unsafe { assume_init(&tmp_buf[4..len]) };
        buf[..len - 4].copy_from_slice(result_buf);
        len - 4
    }

    #[throws]
    #[instrument]
    pub fn set_nonblocking(&mut self, nb: bool) {
        self.socket.set_nonblocking(nb)?;
    }

    #[throws]
    #[instrument]
    pub fn ip_addrs(&self) -> Vec<IpAddr> {
        let mut result: Vec<IpAddr> = vec![];
        let mut addrs: *mut libc::ifaddrs = std::ptr::null_mut();
        let if_name = self.name()?;
        syscall!(getifaddrs(&mut addrs as *mut _))?;
        unsafe {
            while !addrs.is_null() {
                let addr = &*addrs;
                addrs = addr.ifa_next;

                let name = CStr::from_ptr(addr.ifa_name).to_str().unwrap();
                if if_name != name {
                    continue;
                }
                let family = (*addr.ifa_addr).sa_family;
                let addr_len = match family as i32 {
                    libc::AF_INET => mem::size_of::<libc::sockaddr_in>(),
                    libc::AF_INET6 => mem::size_of::<libc::sockaddr_in6>(),
                    _ => continue,
                };

                let (_, sock_addr) = socket2::SockAddr::try_init(|addr_storage, len| {
                    *len = addr_len as u32;
                    std::ptr::copy_nonoverlapping(
                        addr.ifa_addr as *const libc::c_void,
                        addr_storage as *mut _,
                        addr_len,
                    );
                    Ok(())
                })?;

                if let Some(socket_addr) = sock_addr.as_socket() {
                    result.push(socket_addr.ip());
                }
            }
        }
        result
    }
}

#[instrument]
pub fn ifname_to_string(buf: [libc::c_char; libc::IFNAMSIZ]) -> String {
    // TODO: Switch to `CStr::from_bytes_until_nul` when stabilized
    unsafe {
        std::ffi::CStr::from_ptr(buf.as_ptr() as *const _)
            .to_str()
            .unwrap()
            .to_string()
    }
}

#[instrument]
pub fn string_to_ifname(name: &str) -> [libc::c_char; libc::IFNAMSIZ] {
    let mut buf = [0 as libc::c_char; libc::IFNAMSIZ];
    let len = name.len().min(buf.len());
    buf[..len].copy_from_slice(unsafe { &*(name.as_bytes() as *const _ as *const [libc::c_char]) });
    buf
}
