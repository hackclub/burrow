use std::{
    fs::OpenOptions,
    io::{Error, Write},
    mem,
    net::{Ipv4Addr, Ipv6Addr, SocketAddrV4},
    os::{
        fd::RawFd,
        unix::io::{AsRawFd, FromRawFd, IntoRawFd},
    },
};

use fehler::throws;
use libc::in6_ifreq;
use socket2::{Domain, SockAddr, Socket, Type};
use tracing::{info, instrument};

use super::{ifname_to_string, string_to_ifname};
use crate::TunOptions;

mod sys;

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
    pub(crate) fn new_with_options(options: TunOptions) -> TunInterface {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")?;

        let mut flags = libc::IFF_TUN as i16;

        if options.no_pi {
            flags |= libc::IFF_NO_PI as i16;
        }
        if options.tun_excl {
            flags |= libc::IFF_TUN_EXCL as i16;
        }

        let name = options
            .name
            .map(|name| string_to_ifname(&name))
            .unwrap_or([0; libc::IFNAMSIZ]);

        let iff = libc::ifreq {
            ifr_name: name,
            ifr_ifru: libc::__c_anonymous_ifr_ifru { ifru_flags: flags },
        };
        unsafe { sys::tun_set_iff(file.as_raw_fd(), &iff)? };

        let socket = unsafe { socket2::Socket::from_raw_fd(file.into_raw_fd()) };
        TunInterface { socket }
    }

    #[throws]
    #[instrument]
    pub fn name(&self) -> String {
        let mut iff = unsafe { mem::zeroed() };
        unsafe { sys::tun_get_iff(self.socket.as_raw_fd(), &mut iff)? };
        ifname_to_string(iff.ifr_name)
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
    fn in6_ifreq(&self) -> in6_ifreq {
        let mut iff: in6_ifreq = unsafe { mem::zeroed() };
        iff.ifr6_ifindex = self.index()?;
        iff
    }

    #[throws]
    #[instrument]
    pub fn index(&self) -> i32 {
        let mut iff = self.ifreq()?;
        self.perform(|fd| unsafe { sys::if_get_index(fd, &mut iff) })?;
        unsafe { iff.ifr_ifru.ifru_ifindex }
    }

    #[throws]
    #[instrument]
    pub fn set_ipv4_addr(&self, addr: Ipv4Addr) {
        let addr = SockAddr::from(SocketAddrV4::new(addr, 0));
        let mut iff = self.ifreq()?;
        iff.ifr_ifru.ifru_addr = unsafe { *addr.as_ptr() };
        self.perform(|fd| unsafe { sys::if_set_addr(fd, &iff) })?;
        info!("ipv4_addr_set: {:?} (fd: {:?})", addr, self.as_raw_fd())
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
    #[instrument]
    pub fn ipv6_addrs(&self) -> Vec<Ipv6Addr> {
        let ip_addrs = self.ip_addrs()?;
        let mut ipv6_addrs: Vec<Ipv6Addr> = vec![];

        for ip_addr in ip_addrs.iter() {
            if ip_addr.is_ipv6() {
                match ip_addr {
                    IpAddr::V6(addr) => {
                        ipv6_addrs.push(*addr);
                    }
                    _ => {}
                }
            }
        }
        ipv6_addrs
    }

    #[throws]
    #[instrument]
    pub fn set_broadcast_addr(&self, addr: Ipv4Addr) {
        let addr = SockAddr::from(SocketAddrV4::new(addr, 0));
        let mut iff = self.ifreq()?;
        iff.ifr_ifru.ifru_broadaddr = unsafe { *addr.as_ptr() };
        self.perform(|fd| unsafe { sys::if_set_brdaddr(fd, &iff) })?;
        info!(
            "broadcast_addr_set: {:?} (fd: {:?})",
            addr,
            self.as_raw_fd()
        )
    }

    #[throws]
    #[instrument]
    pub fn broadcast_addr(&self) -> Ipv4Addr {
        let mut iff = self.ifreq()?;
        self.perform(|fd| unsafe { sys::if_get_brdaddr(fd, &mut iff) })?;
        let addr =
            unsafe { *(&iff.ifr_ifru.ifru_broadaddr as *const _ as *const sys::sockaddr_in) };
        Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr))
    }

    #[throws]
    #[instrument]
    pub fn set_ipv6_addr(&self, addr: Ipv6Addr) {
        let mut iff = self.in6_ifreq()?;
        iff.ifr6_addr.s6_addr = addr.octets();
        self.perform6(|fd| unsafe { sys::if_set_addr6(fd, &iff) })?;
        info!("ipv6_addr_set: {:?} (fd: {:?})", addr, self.as_raw_fd())
    }

    #[throws]
    #[instrument]
    pub fn set_mtu(&self, mtu: i32) {
        let mut iff = self.ifreq()?;
        iff.ifr_ifru.ifru_mtu = mtu;
        self.perform(|fd| unsafe { sys::if_set_mtu(fd, &iff) })?;
        info!("mtu_set: {:?} (fd: {:?})", mtu, self.as_raw_fd())
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
    pub fn set_netmask(&self, addr: Ipv4Addr) {
        let addr = SockAddr::from(SocketAddrV4::new(addr, 0));

        let mut iff = self.ifreq()?;
        iff.ifr_ifru.ifru_netmask = unsafe { *addr.as_ptr() };

        self.perform(|fd| unsafe { sys::if_set_netmask(fd, &iff) })?;

        info!(
            "netmask_set: {:?} (fd: {:?})",
            unsafe { iff.ifr_ifru.ifru_netmask },
            self.as_raw_fd()
        )
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
    fn perform<R>(&self, perform: impl FnOnce(RawFd) -> Result<R, nix::Error>) -> R {
        let span = tracing::info_span!("perform");
        let _enter = span.enter();

        let socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        perform(socket.as_raw_fd())?
    }

    #[throws]
    fn perform6<R>(&self, perform: impl FnOnce(RawFd) -> Result<R, nix::Error>) -> R {
        let span = tracing::info_span!("perform");
        let _enter = span.enter();

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, None)?;
        perform(socket.as_raw_fd())?
    }

    #[throws]
    #[instrument]
    pub fn send(&self, buf: &[u8]) -> usize {
        self.socket.send(buf)?
    }
}
