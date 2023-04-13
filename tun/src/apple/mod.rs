use socket2::SockAddr;
use std::io::Result;
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

mod kern_control;
mod queue;

pub use queue::TunQueue;

use crate::syscall;
use crate::unix::copy_if_name;
use kern_control::SysControlSocket;

pub struct TunInterface {
    socket: socket2::Socket,
}

impl TunInterface {
    pub fn new() -> Result<TunInterface> {
        TunInterface::connect(None)
    }

    fn connect(addr: Option<SockAddr>) -> Result<TunInterface> {
        use socket2::{Domain, Protocol, Socket, Type};

        let socket = Socket::new(
            Domain::from(libc::AF_SYSTEM),
            Type::DGRAM,
            Some(Protocol::from(libc::SYSPROTO_CONTROL)),
        )?;
        let addr = match addr {
            Some(addr) => addr,
            None => socket.resolve(sys::UTUN_CONTROL_NAME, 0)?,
        };
        socket.connect(&addr)?;

        Ok(TunInterface { socket })
    }

    pub fn name(&self) -> Result<String> {
        let mut buf = [0i8; libc::IFNAMSIZ];
        let mut len = buf.len() as libc::socklen_t;
        syscall!(getsockopt(
            self.as_raw_fd(),
            libc::SYSPROTO_CONTROL,
            sys::UTUN_OPT_IFNAME,
            buf.as_mut_ptr() as *mut libc::c_void,
            &mut len,
        ))?;
        let name = copy_if_name(buf);
        Ok(name)
    }

    pub fn queue(&self) -> Result<TunQueue> {
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

mod sys {
    pub const UTUN_CONTROL_NAME: &str = "com.apple.net.utun_control";

    pub const UTUN_OPT_IFNAME: libc::c_int = 2;

    /// Copied from https://github.com/rust-lang/socket2/blob/61314a231f73964b3db969ef72c0e9479df320f3/src/sys/unix.rs#L168-L178
    /// getsockopt is not exposed by socket2
    #[macro_export]
    macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        #[allow(unused_unsafe)]
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
    }
}
