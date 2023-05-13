use fehler::throws;
use libc::c_char;
use socket2::{Domain, SockAddr, Socket, Type};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::fd::{AsRawFd, RawFd};
use std::{io::Error, mem};

mod kern_control;
mod sys;

pub use super::queue::TunQueue;

use super::{ifname_to_string, string_to_ifname};
use kern_control::SysControlSocket;

#[derive(Debug)]
pub struct TunInterface {
    pub(crate) socket: socket2::Socket,
}

impl TunInterface {
    #[throws]
    pub fn new() -> TunInterface {
        TunInterface::connect(0)?
    }

    #[throws]
    fn connect(index: u32) -> TunInterface {
        use socket2::{Domain, Protocol, Socket, Type};

        let socket = Socket::new(
            Domain::from(sys::AF_SYSTEM),
            Type::DGRAM,
            Some(Protocol::from(sys::SYSPROTO_CONTROL)),
        )?;
        let addr = socket.resolve(sys::UTUN_CONTROL_NAME, index)?;
        socket.connect(&addr)?;

        TunInterface { socket }
    }

    #[throws]
    pub fn name(&self) -> String {
        let mut buf = [0 as c_char; sys::IFNAMSIZ];
        let mut len = buf.len() as sys::socklen_t;
        sys::syscall!(getsockopt(
            self.as_raw_fd(),
            sys::SYSPROTO_CONTROL,
            sys::UTUN_OPT_IFNAME,
            buf.as_mut_ptr() as *mut sys::c_void,
            &mut len,
        ))?;
        ifname_to_string(buf)
    }

    #[throws]
    fn ifreq(&self) -> sys::ifreq {
        let mut iff: sys::ifreq = unsafe { mem::zeroed() };
        iff.ifr_name = string_to_ifname(&self.name()?);
        iff
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
        let addr = unsafe { *(&iff.ifr_ifru.ifru_addr as *const _ as *const sys::sockaddr_in) };
        Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr))
    }

    #[throws]
    fn perform<R>(&self, perform: impl FnOnce(RawFd) -> Result<R, nix::Error>) -> R {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        perform(socket.as_raw_fd())?
    }
}
