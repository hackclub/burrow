use std::mem;

use libc::{c_char, c_int, c_short, c_uint, c_ulong, sockaddr, sockaddr_in6, time_t};
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
pub const SCOPE6_ID_MAX: usize = 16;

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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct in6_addrlifetime{
    pub ia6t_expire: time_t,
    pub ia6t_preferred: time_t,
    pub ia6t_vltime: u32,
    pub ia6t_pltime: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct in6_ifstat {
    pub ifs6_in_receive: u64,
    pub ifs6_in_hdrerr: u64,
    pub ifs6_in_toobig: u64,
    pub ifs6_in_noroute: u64,
    pub ifs6_in_addrerr: u64,
    pub ifs6_in_protounknown: u64,
    pub ifs6_in_truncated: u64,
    pub ifs6_in_discard: u64,
    pub ifs6_in_deliver: u64,
    pub ifs6_out_forward: u64,
    pub ifs6_out_request: u64,
    pub ifs6_out_discard: u64,
    pub ifs6_out_fragok: u64,
    pub ifs6_out_fragfail: u64,
    pub ifs6_out_fragcreat: u64,
    pub ifs6_reass_reqd: u64,
    pub ifs6_reass_ok: u64,
    pub ifs6_atmfrag_rcvd: u64,
    pub ifs6_reass_fail: u64,
    pub ifs6_in_mcast: u64,
    pub ifs6_out_mcast: u64,
    pub ifs6_cantfoward_icmp6: u64,
    pub ifs6_addr_expiry_cnt: u64,
    pub ifs6_pfx_expiry_cnt: u64,
    pub ifs6_defrtr_expiry_cnt: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct icmp6_ifstat {
    pub ifs6_in_msg: u64,
    pub ifs6_in_error: u64,
    pub ifs6_in_dstunreach: u64,
    pub ifs6_in_adminprohib: u64,
    pub ifs6_in_timeexceed: u64,
    pub ifs6_in_paramprob: u64,
    pub ifs6_in_pkttoobig: u64,
    pub ifs6_in_echo: u64,
    pub ifs6_in_echoreply: u64,
    pub ifs6_in_routersolicit: u64,
    pub ifs6_in_routeradvert: u64,
    pub ifs6_in_neighborsolicit: u64,
    pub ifs6_in_neighboradvert: u64,
    pub ifs6_in_redirect: u64,
    pub ifs6_in_mldquery: u64,
    pub ifs6_in_mldreport: u64,
    pub ifs6_in_mlddone: u64,
    pub ifs6_out_msg: u64,
    pub ifs6_out_error: u64,
    pub ifs6_out_dstunreach: u64,
    pub ifs6_out_adminprohib: u64,
    pub ifs6_out_timeexceed: u64,
    pub ifs6_out_paramprob: u64,
    pub ifs6_out_pkttoobig: u64,
    pub ifs6_out_echo: u64,
    pub ifs6_out_echoreply: u64,
    pub ifs6_out_routersolicit: u64,
    pub ifs6_out_routeradvert: u64,
    pub ifs6_out_neighborsolicit: u64,
    pub ifs6_out_neighboradvert: u64,
    pub ifs6_out_redirect: u64,
    pub ifs6_out_mldquery: u64,
    pub ifs6_out_mldreport: u64,
    pub ifs6_out_mlddone: u64,
}

#[repr(C)]
pub union ifr_ifru6 {
    pub ifru_addr: sockaddr_in6,
    pub ifru_dstaddr: sockaddr_in6,
    pub ifru_flags: c_int,
    pub ifru_flags6: c_int,
    pub ifru_metric: c_int,
    pub ifru_intval: c_int,
    pub ifru_data: *mut c_char,
    pub ifru_lifetime: in6_addrlifetime, // ifru_lifetime
    pub ifru_stat: in6_ifstat,
    pub ifru_icmp6stat: icmp6_ifstat,
    pub ifru_scope_id: [u32; SCOPE6_ID_MAX]
}

#[repr(C)]
pub struct in6_ifreq {
    pub ifr_name: [c_char; IFNAMSIZ],
    pub ifr_ifru: ifr_ifru6,
}

pub const SIOCSIFADDR: c_ulong = request_code_write!(b'i', 12, mem::size_of::<ifreq>());
pub const SIOCSIFADDR_IN6: c_ulong = request_code_write!(b'i', 12, mem::size_of::<in6_ifreq>());
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
ioctl_write_ptr_bad!(if_set_addr6, SIOCSIFADDR_IN6, in6_ifreq);
ioctl_write_ptr_bad!(if_set_mtu, SIOCSIFMTU, ifreq);
ioctl_write_ptr_bad!(if_set_netmask, SIOCSIFNETMASK, ifreq);
