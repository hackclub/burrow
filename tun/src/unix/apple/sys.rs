use std::mem;

use libc::{c_char, c_int, c_short, c_uint, c_ulong, sockaddr};
pub use libc::{
    c_void,
    sockaddr_ctl,
    sockaddr_in,
    socklen_t,
    AF_SYSTEM,
    AF_SYS_CONTROL,
    IFNAMSIZ,
    SYSPROTO_CONTROL,
};
use nix::{
    ioctl_read_bad,
    ioctl_readwrite,
    ioctl_write_ptr_bad,
    request_code_readwrite,
    request_code_write,
};

pub const UTUN_CONTROL_NAME: &str = "com.apple.net.utun_control";
pub const UTUN_OPT_IFNAME: libc::c_int = 2;

pub const MAX_KCTL_NAME: usize = 96;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ctl_info {
    pub ctl_id: u32,
    pub ctl_name: [u8; MAX_KCTL_NAME],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ifkpi {
    pub ifk_module_id: c_uint,
    pub ifk_type: c_uint,
    pub ifk_ptr: *mut c_void,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ifdevmtu {
    pub ifdm_current: c_int,
    pub ifdm_min: c_int,
    pub ifdm_max: c_int,
}

#[repr(C)]
pub union ifr_ifru {
    pub ifru_addr: sockaddr,
    pub ifru_dstaddr: sockaddr,
    pub ifru_broadaddr: sockaddr,
    pub ifru_netmask: sockaddr,
    pub ifru_flags: c_short,
    pub ifru_metric: c_int,
    pub ifru_mtu: c_int,
    pub ifru_phys: c_int,
    pub ifru_media: c_int,
    pub ifru_intval: c_int,
    pub ifru_data: *mut c_char,
    pub ifru_devmtu: ifdevmtu,
    pub ifru_kpi: ifkpi,
    pub ifru_wake_flags: u32,
    pub ifru_route_refcnt: u32,
    pub ifru_cap: [c_int; 2],
    pub ifru_functional_type: u32,
}

#[repr(C)]
pub struct ifreq {
    pub ifr_name: [c_char; IFNAMSIZ],
    pub ifr_ifru: ifr_ifru,
}

pub const SIOCSIFADDR: c_ulong = request_code_write!(b'i', 12, mem::size_of::<ifreq>());
pub const SIOCGIFMTU: c_ulong = request_code_readwrite!(b'i', 51, mem::size_of::<ifreq>());
pub const SIOCSIFMTU: c_ulong = request_code_write!(b'i', 52, mem::size_of::<ifreq>());
pub const SIOCGIFNETMASK: c_ulong = request_code_readwrite!(b'i', 37, mem::size_of::<ifreq>());
pub const SIOCSIFNETMASK: c_ulong = request_code_write!(b'i', 22, mem::size_of::<ifreq>());

#[macro_export]
macro_rules! syscall {
    ($call: ident ( $($arg: expr),* $(,)* ) ) => {{
        match unsafe { ::libc::$call($($arg, )*) } {
            -1 => Err(::std::io::Error::last_os_error()),
            res => Ok(res),
        }
    }};
}

pub use syscall;

ioctl_readwrite!(resolve_ctl_info, b'N', 3, ctl_info);
ioctl_read_bad!(if_get_addr, libc::SIOCGIFADDR, ifreq);
ioctl_read_bad!(if_get_mtu, SIOCGIFMTU, ifreq);
ioctl_read_bad!(if_get_netmask, SIOCGIFNETMASK, ifreq);
ioctl_write_ptr_bad!(if_set_addr, SIOCSIFADDR, ifreq);
ioctl_write_ptr_bad!(if_set_mtu, SIOCSIFMTU, ifreq);
ioctl_write_ptr_bad!(if_set_netmask, SIOCSIFNETMASK, ifreq);
