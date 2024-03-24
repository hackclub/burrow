use std::{
    io::{Error, IoSlice},
    mem,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4},
    os::fd::{AsRawFd, FromRawFd, RawFd},
};

use byteorder::{ByteOrder, NetworkEndian};
use fehler::throws;
use libc::{c_char, iovec, writev, AF_INET, AF_INET6};
use socket2::{Domain, SockAddr, Socket, Type};
use tracing::{self, instrument};

pub mod kern_control;
pub mod sys;

use kern_control::SysControlSocket;

use super::{ifname_to_string, string_to_ifname};
use crate::TunOptions;

#[derive(Debug)]
pub struct TunInterface {
    pub(crate) socket: socket2::Socket,
}

impl TunInterface {
    #[throws]
    #[instrument]
    pub fn new() -> TunInterface {
        Self::new_with_options(TunOptions::new())?
    }

    #[throws]
    #[instrument]
    pub fn new_with_options(options: TunOptions) -> TunInterface {
        let ti = if options.tun_retrieve {
            TunInterface::retrieve().ok_or(Error::new(
                std::io::ErrorKind::NotFound,
                "No tun interface found",
            ))?
        } else {
            TunInterface::connect(0)?
        };
        ti.configure(options)?;
        ti
    }

    pub fn retrieve() -> Option<TunInterface> {
        (3..100)
            .filter_map(|fd| unsafe {
                let peer_addr = socket2::SockAddr::try_init(|storage, len| {
                    *len = mem::size_of::<sys::sockaddr_ctl>() as u32;
                    libc::getpeername(fd, storage as *mut _, len);
                    Ok(())
                })
                .map(|(_, addr)| (fd, addr));
                peer_addr.ok()
            })
            .filter(|(_fd, addr)| {
                let ctl_addr = unsafe { &*(addr.as_ptr() as *const libc::sockaddr_ctl) };
                addr.family() == libc::AF_SYSTEM as u8
                    && ctl_addr.ss_sysaddr == libc::AF_SYS_CONTROL as u16
            })
            .map(|(fd, _)| {
                let socket = unsafe { socket2::Socket::from_raw_fd(fd) };
                TunInterface { socket }
            })
            .next()
    }

    #[throws]
    fn configure(&self, options: TunOptions) {
        for addr in options.address {
            if let Ok(addr) = addr.parse::<IpAddr>() {
                match addr {
                    IpAddr::V4(addr) => self.set_ipv4_addr(addr)?,
                    IpAddr::V6(addr) => self.set_ipv6_addr(addr)?,
                }
            }
        }
    }

    #[throws]
    #[instrument]
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
    #[instrument]
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
    #[instrument]
    fn ifreq(&self) -> sys::ifreq {
        let mut iff: sys::ifreq = unsafe { mem::zeroed() };
        iff.ifr_name = string_to_ifname(&self.name()?);
        iff
    }

    #[throws]
    #[instrument]
    fn in6_ifreq(&self) -> sys::in6_ifreq {
        let mut iff: sys::in6_ifreq = unsafe { mem::zeroed() };
        iff.ifr_name = string_to_ifname(&self.name()?);
        iff
    }

    #[throws]
    #[instrument]
    pub fn set_ipv4_addr(&self, addr: Ipv4Addr) {
        let addr = SockAddr::from(SocketAddrV4::new(addr, 0));
        let mut iff = self.ifreq()?;
        iff.ifr_ifru.ifru_addr = unsafe { *addr.as_ptr() };
        self.perform(|fd| unsafe { sys::if_set_addr(fd, &iff) })?;
        tracing::info!("ipv4_addr_set: {:?} (fd: {:?})", addr, self.as_raw_fd())
    }

    #[throws]
    #[instrument]
    pub fn ipv4_addr(&self) -> Ipv4Addr {
        let mut iff = self.ifreq()?;
        self.perform(|fd| unsafe { sys::if_get_addr(fd, &mut iff) })?;
        let addr = unsafe { *(&iff.ifr_ifru.ifru_addr as *const _ as *const sys::sockaddr_in) };
        Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr))
    }

    #[throws]
    pub fn set_ipv6_addr(&self, _addr: Ipv6Addr) {
        // let addr = SockAddr::from(SocketAddrV6::new(addr, 0, 0, 0));
        // println!("addr: {:?}", addr);
        // let mut iff = self.in6_ifreq()?;
        // let sto = addr.as_storage();
        // let ifadddr_ptr: *const sockaddr_in6 = addr_of!(sto).cast();
        // iff.ifr_ifru.ifru_addr = unsafe { *ifadddr_ptr };
        // println!("ifru addr set");
        // println!("{:?}", sys::SIOCSIFADDR_IN6);
        // self.perform6(|fd| unsafe { sys::if_set_addr6(fd, &iff) })?;
        // tracing::info!("ipv6_addr_set");
        tracing::warn!("Setting IPV6 address on MacOS CLI mode is not supported yet.");
    }

    #[throws]
    fn perform<R>(&self, perform: impl FnOnce(RawFd) -> Result<R, nix::Error>) -> R {
        let span = tracing::info_span!("perform", fd = self.as_raw_fd());
        let _enter = span.enter();

        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        perform(socket.as_raw_fd())?
    }

    #[throws]
    fn perform6<R>(&self, perform: impl FnOnce(RawFd) -> Result<R, nix::Error>) -> R {
        let span = tracing::info_span!("perform6", fd = self.as_raw_fd());
        let _enter = span.enter();

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, None)?;
        perform(socket.as_raw_fd())?
    }

    #[throws]
    #[instrument]
    pub fn mtu(&self) -> i32 {
        let mut iff = self.ifreq()?;
        self.perform(|fd| unsafe { sys::if_get_mtu(fd, &mut iff) })?;
        let mtu = unsafe { iff.ifr_ifru.ifru_mtu };

        mtu
    }

    #[throws]
    #[instrument]
    pub fn set_mtu(&self, mtu: i32) {
        let mut iff = self.ifreq()?;
        iff.ifr_ifru.ifru_mtu = mtu;
        self.perform(|fd| unsafe { sys::if_set_mtu(fd, &iff) })?;
        tracing::info!("mtu_set: {:?} (fd: {:?})", mtu, self.as_raw_fd())
    }

    #[throws]
    #[instrument]
    pub fn netmask(&self) -> Ipv4Addr {
        let mut iff = self.ifreq()?;
        self.perform(|fd| unsafe { sys::if_get_netmask(fd, &mut iff) })?;

        let netmask =
            unsafe { *(&iff.ifr_ifru.ifru_netmask as *const _ as *const sys::sockaddr_in) };

        Ipv4Addr::from(u32::from_be(netmask.sin_addr.s_addr))
    }

    #[throws]
    #[instrument]
    pub fn set_netmask(&self, addr: Ipv4Addr) {
        let addr = SockAddr::from(SocketAddrV4::new(addr, 0));
        let mut iff = self.ifreq()?;
        iff.ifr_ifru.ifru_netmask = unsafe { *addr.as_ptr() };
        self.perform(|fd| unsafe { sys::if_set_netmask(fd, &iff) })?;
        tracing::info!(
            "netmask_set: {:?} (fd: {:?})",
            unsafe { iff.ifr_ifru.ifru_netmask },
            self.as_raw_fd()
        )
    }

    #[throws]
    #[instrument]
    pub fn send(&self, buf: &[u8]) -> usize {
        use std::io::ErrorKind;
        let proto = match buf[0] >> 4 {
            6 => Ok(AF_INET6),
            4 => Ok(AF_INET),
            _ => Err(Error::new(ErrorKind::InvalidInput, "Invalid IP version")),
        }?;
        let mut pbuf = [0; 4];
        NetworkEndian::write_i32(&mut pbuf, proto);

        let bufs = [IoSlice::new(&pbuf), IoSlice::new(buf)];
        let bytes_written: isize = unsafe {
            writev(
                self.as_raw_fd(),
                bufs.as_ptr() as *const iovec,
                bufs.len() as i32,
            )
        };
        bytes_written
            .try_into()
            .map_err(|_| Error::new(ErrorKind::Other, "Conversion error"))?
    }
}
