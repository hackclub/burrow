use fehler::throws;

use socket2::{Domain, SockAddr, Socket, Type};
use std::fs::OpenOptions;
use std::io::Error;
use std::mem;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use std::os::fd::RawFd;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};
use std::ptr;
use std::sync::{Arc, Mutex};

use super::{ifname_to_string, string_to_ifname};
use crate::TunInterface;

mod sys;

#[derive(Debug)]
pub struct PlatformTun {
    pub(crate) socket: socket2::Socket,
}

impl PlatformTun {
    #[throws]
    pub fn index(&self) -> i32 {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        unsafe {
            let mut iff: libc::ifreq = mem::zeroed();
            iff.ifr_name = string_to_ifname(&self.get_interface_name()?);
            sys::if_get_index(socket.as_raw_fd(), &mut iff)?;
            iff.ifr_ifru.ifru_ifindex
        }
    }
    #[throws]
    pub(crate) fn new() -> PlatformTun {
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

        let socket = unsafe { socket2::Socket::from_raw_fd(file.into_raw_fd()) };
        PlatformTun { socket }
    }
}

impl TunInterface for PlatformTun {
    #[throws]
    fn get_interface_name(&self) -> String {
        unsafe {
            let mut iff = mem::zeroed();
            sys::tun_get_iff(self.socket.as_raw_fd(), &mut iff)?;
            ifname_to_string(iff.ifr_name)
        }
    }

    #[throws]
    fn set_ip(&self, addr: IpAddr) {
        match addr {
            IpAddr::V4(addr) => self.set_ipv4(addr)?,
            _ => (),
        }
    }

    #[throws]
    fn set_ipv4(&self, addr: Ipv4Addr) {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        let addr = SockAddr::from(SocketAddrV4::new(addr, 0));
        unsafe {
            let mut iff: libc::ifreq = mem::zeroed();
            iff.ifr_name = string_to_ifname(&self.get_interface_name()?);
            iff.ifr_ifru.ifru_addr = *addr.as_ptr();
            sys::if_set_addr(socket.as_raw_fd(), &iff)?;
        }
    }

    #[throws]
    fn get_ip(&self) -> IpAddr {
        let addr = self.socket.local_addr()?;
        addr.as_socket().unwrap().ip()
    }

    fn into_raw_socket(self) -> socket2::Socket {
        self.socket
    }
}
