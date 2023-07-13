use std::{
    io::{Error, Read},
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
};
use tracing::instrument;

use super::TunOptions;

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
        TunInterface {
            socket: socket2::Socket::from_raw_fd(fd),
        }
    }
}

impl IntoRawFd for TunInterface {
    fn into_raw_fd(self) -> RawFd {
        self.socket.into_raw_fd()
    }
}

impl TunInterface {
    #[throws]
    #[instrument]
    pub fn recv(&mut self, buf: &mut [u8]) -> usize {
        self.socket.read(buf)?
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

#[cfg(test)]
mod test {

    use super::*;

    use std::net::Ipv4Addr;

    #[throws]
    #[test]
    fn tst_read() {
        // This test is interactive, you need to send a packet to any server through 192.168.1.10
        // EG. `sudo route add 8.8.8.8 192.168.1.10`,
        //`dig @8.8.8.8 hackclub.com`
        let mut tun = TunInterface::new()?;
        println!("tun name: {:?}", tun.name()?);
        tun.set_ipv4_addr(Ipv4Addr::from([192, 168, 1, 10]))?;
        println!("tun ip: {:?}", tun.ipv4_addr()?);
        println!("Waiting for a packet...");
        let buf = &mut [0u8; 1500];
        let res = tun.recv(buf);
        println!("Received!");
        assert!(res.is_ok());
    }

    #[test]
    #[throws]
    fn write_packets() {
        let tun = TunInterface::new()?;
        let mut buf = [0u8; 1500];
        buf[0] = 6 << 4;
        let bytes_written = tun.send(&buf)?;
        assert_eq!(bytes_written, 1504);
    }
}
