use std::mem::size_of;

pub use libc::{ifreq, sockaddr, sockaddr_in, sockaddr_in6};
use nix::{ioctl_read_bad, ioctl_write_ptr_bad, request_code_read, request_code_write};

ioctl_write_ptr_bad!(
    tun_set_iff,
    request_code_write!(b'T', 202, size_of::<libc::c_int>()),
    libc::ifreq
);
ioctl_read_bad!(
    tun_get_iff,
    request_code_read!(b'T', 210, size_of::<libc::c_uint>()),
    libc::ifreq
);
ioctl_read_bad!(if_get_index, libc::SIOCGIFINDEX, libc::ifreq);
ioctl_read_bad!(if_get_addr, libc::SIOCGIFADDR, libc::ifreq);
ioctl_read_bad!(if_get_brdaddr, libc::SIOCGIFBRDADDR, libc::ifreq);
ioctl_read_bad!(if_get_mtu, libc::SIOCGIFMTU, libc::ifreq);
ioctl_read_bad!(if_get_netmask, libc::SIOCGIFNETMASK, libc::ifreq);

ioctl_write_ptr_bad!(if_set_addr, libc::SIOCSIFADDR, libc::ifreq);
ioctl_write_ptr_bad!(if_set_addr6, libc::SIOCSIFADDR, libc::in6_ifreq);
ioctl_write_ptr_bad!(if_set_brdaddr, libc::SIOCSIFBRDADDR, libc::ifreq);
ioctl_write_ptr_bad!(if_set_mtu, libc::SIOCSIFMTU, libc::ifreq);
ioctl_write_ptr_bad!(if_set_netmask, libc::SIOCSIFNETMASK, libc::ifreq);
