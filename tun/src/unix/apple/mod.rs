use fehler::throws;
use libc::c_char;
use socket2::{Domain, SockAddr};
use std::mem;
use std::net::SocketAddrV4;
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

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
            Domain::from(libc::AF_SYSTEM),
            Type::DGRAM,
            Some(Protocol::from(libc::SYSPROTO_CONTROL)),
        )?;
        let addr = socket.resolve(sys::UTUN_CONTROL_NAME, index)?;
        socket.connect(&addr)?;

        TunInterface { socket }
    }

    #[throws]
    pub fn name(&self) -> String {
        let mut buf = [0 as c_char; libc::IFNAMSIZ];
        let mut len = buf.len() as libc::socklen_t;
        sys::syscall!(getsockopt(
            self.as_raw_fd(),
            libc::SYSPROTO_CONTROL,
            sys::UTUN_OPT_IFNAME,
            buf.as_mut_ptr() as *mut libc::c_void,
            &mut len,
        ))?;
        ifname_to_string(buf)
    }

    #[throws]
    pub fn index(&self) -> i32 {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        unsafe {
            let mut iff: libc::ifreq = mem::zeroed();
            iff.ifr_name = string_to_ifname(self.name()?);
            sys::if_get_index(socket.as_raw_fd(), &mut iff)?;
            iff.ifr_ifru.ifru_ifindex
        }
    }

    #[throws]
    pub fn set_addr(&self, addr: IpAddr) {
        match addr {
            IpAddr::V4(addr) => self.set_ipv4_addr(addr)?,
            _ => (),
        }
    }

    #[throws]
    pub fn set_ipv4_addr(&self, addr: Ipv4Addr) {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        let addr = SockAddr::from(SocketAddrV4::new(addr, 0));
        unsafe {
            let mut iff: libc::ifreq = mem::zeroed();
            iff.ifr_name = string_to_ifname(self.name()?);
            iff.ifr_ifru.ifru_addr = *addr.as_ptr();
            sys::if_set_addr(socket.as_raw_fd(), &iff)?;
        }
    }
}

impl AsRawFd for TunInterface {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl IntoRawFd for TunInterface {
    fn into_raw_fd(self) -> RawFd {
        self.socket.into_raw_fd()
    }
}
