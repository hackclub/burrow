use fehler::throws;

use socket2::{Domain, SockAddr, Socket, Type};
use std::fs::OpenOptions;
use std::io::Error;
use std::mem;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::fd::RawFd;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};


use super::{ifname_to_string, string_to_ifname};

mod sys;

#[derive(Debug)]
pub struct TunInterface {
    pub(crate) socket: socket2::Socket,
}

impl TunInterface {
    #[throws]
    pub fn new() -> TunInterface {
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
        TunInterface { socket }
    }

    #[throws]
    pub fn name(&self) -> String {
        let mut iff = unsafe { mem::zeroed() };
        unsafe { sys::tun_get_iff(self.socket.as_raw_fd(), &mut iff)? };
        ifname_to_string(iff.ifr_name)
    }

    #[throws]
    fn ifreq(&self) -> sys::ifreq {
        let mut iff: sys::ifreq = unsafe { mem::zeroed() };
        iff.ifr_name = string_to_ifname(&self.name()?);
        iff
    }

    #[throws]
    pub fn index(&self) -> i32 {
        let mut iff = self.ifreq()?;
        self.perform(|fd| unsafe { sys::if_get_index(fd, &mut iff) })?;
        unsafe { iff.ifr_ifru.ifru_ifindex }
    }

    #[throws]
    pub fn set_ipv4_addr(&self, addr: Ipv4Addr) {
        let addr = SockAddr::from(SocketAddrV4::new(addr, 0));

        let mut iff = self.ifreq()?;
        iff.ifr_ifru.ifru_addr = unsafe { *addr.as_ptr() };

        self.perform(|fd| unsafe { sys::if_set_addr(fd, &iff) })?;
    }

    #[throws]
    pub fn ipv4_addr(&self) -> Ipv4Addr {
        let mut iff = self.ifreq()?;
        self.perform(|fd| unsafe { sys::if_get_addr(fd, &mut iff) })?;

        let addr = unsafe { &*(&iff.ifr_ifru.ifru_addr as *const _ as *const sys::sockaddr_in) };

        Ipv4Addr::from(addr.sin_addr.s_addr)
    }

    #[throws]
    fn perform<R>(&self, perform: impl FnOnce(RawFd) -> Result<R, nix::Error>) -> R {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        perform(socket.as_raw_fd())?
    }
}
