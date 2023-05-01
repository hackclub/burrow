use fehler::throws;
use libc::c_char;
use std::io::Error;
use std::net::Ipv4Addr;
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

mod kern_control;
mod sys;

pub use super::queue::TunQueue;

use super::ifname_to_string;
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
    pub fn set_ipv4_addr(&self, _addr: Ipv4Addr) {
        todo!()
    }

    #[throws]
    pub fn ipv4_addr(&self) -> Ipv4Addr {
        todo!()
    }

    #[throws]
    pub async fn reroute(&mut self, interface_addr: Ipv4Addr, dest: Ipv4Addr, gateway: Ipv4Addr) {
        todo!()
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
