pub const UTUN_CONTROL_NAME: &str = "com.apple.net.utun_control";
pub const UTUN_OPT_IFNAME: libc::c_int = 2;

#[repr(C)]
pub struct ifreq {}

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
