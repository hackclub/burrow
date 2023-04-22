pub use libc::{c_void, socklen_t, SYSPROTO_CONTROL, IFNAMSIZ, sockaddr_ctl, AF_SYSTEM, AF_SYS_CONTROL};
use nix::ioctl_readwrite;

pub const UTUN_CONTROL_NAME: &str = "com.apple.net.utun_control";
pub const UTUN_OPT_IFNAME: libc::c_int = 2;

pub const MAX_KCTL_NAME: usize = 96;

    #[repr(C)]
pub struct ctl_info {
    pub ctl_id: u32,
    pub ctl_name: [u8; MAX_KCTL_NAME],
}

#[repr(C)]
pub struct ifreq {}

ioctl_readwrite!(resolve_ctl_info, b'N', 3, ctl_info);

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

pub use syscall;
