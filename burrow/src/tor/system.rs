use std::net::SocketAddr;

use anyhow::{Context, Result};
use tokio::net::{TcpListener, TcpStream};

use super::SystemTcpStackConfig;

pub struct SystemTcpStackRuntime {
    listener: TcpListener,
}

impl SystemTcpStackRuntime {
    pub async fn bind(config: &SystemTcpStackConfig) -> Result<Self> {
        let listener = TcpListener::bind(&config.listen)
            .await
            .with_context(|| format!("failed to bind transparent listener on {}", config.listen))?;
        Ok(Self { listener })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .expect("listener should always have a local address")
    }

    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr)> {
        let (stream, _) = self
            .listener
            .accept()
            .await
            .context("failed to accept transparent listener connection")?;
        let original_dst = original_destination(&stream)?;
        Ok((stream, original_dst))
    }
}

#[cfg(target_os = "linux")]
fn original_destination(stream: &TcpStream) -> Result<SocketAddr> {
    use std::{
        mem::{size_of, MaybeUninit},
        os::fd::AsRawFd,
    };

    let level = if stream.local_addr()?.is_ipv6() {
        libc::SOL_IPV6
    } else {
        libc::SOL_IP
    };

    let mut addr = MaybeUninit::<libc::sockaddr_storage>::zeroed();
    let mut len = size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    let rc = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            level,
            80,
            addr.as_mut_ptr().cast(),
            &mut len,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error()).context("SO_ORIGINAL_DST lookup failed");
    }

    socket_addr_from_storage(unsafe { &addr.assume_init() }, len as usize)
}

#[cfg(not(target_os = "linux"))]
fn original_destination(_stream: &TcpStream) -> Result<SocketAddr> {
    anyhow::bail!("system tcp stack transparent destination lookup is only implemented on linux")
}

fn socket_addr_from_storage(addr: &libc::sockaddr_storage, len: usize) -> Result<SocketAddr> {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

    if len < std::mem::size_of::<libc::sa_family_t>() {
        anyhow::bail!("socket address buffer was too short");
    }

    match addr.ss_family as i32 {
        libc::AF_INET => {
            let addr_in = unsafe { *(addr as *const _ as *const libc::sockaddr_in) };
            let ip = Ipv4Addr::from(u32::from_be(addr_in.sin_addr.s_addr));
            let port = u16::from_be(addr_in.sin_port);
            Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
        }
        libc::AF_INET6 => {
            let addr_in = unsafe { *(addr as *const _ as *const libc::sockaddr_in6) };
            let ip = Ipv6Addr::from(addr_in.sin6_addr.s6_addr);
            let port = u16::from_be(addr_in.sin6_port);
            Ok(SocketAddr::V6(SocketAddrV6::new(
                ip,
                port,
                addr_in.sin6_flowinfo,
                addr_in.sin6_scope_id,
            )))
        }
        family => anyhow::bail!("unsupported socket address family {family}"),
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::{
        mem::size_of,
        net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6},
    };

    #[test]
    fn parses_ipv4_socket_addr() {
        let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
        let addr_in = unsafe { &mut *(&mut storage as *mut _ as *mut libc::sockaddr_in) };
        addr_in.sin_family = libc::AF_INET as libc::sa_family_t;
        addr_in.sin_port = u16::to_be(9040);
        addr_in.sin_addr = libc::in_addr {
            s_addr: u32::to_be(u32::from(Ipv4Addr::new(127, 0, 0, 1))),
        };

        let parsed = socket_addr_from_storage(&storage, size_of::<libc::sockaddr_in>()).unwrap();
        assert_eq!(
            parsed,
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 9040))
        );
    }

    #[test]
    fn parses_ipv6_socket_addr() {
        let mut storage = unsafe { std::mem::zeroed::<libc::sockaddr_storage>() };
        let addr_in = unsafe { &mut *(&mut storage as *mut _ as *mut libc::sockaddr_in6) };
        addr_in.sin6_family = libc::AF_INET6 as libc::sa_family_t;
        addr_in.sin6_port = u16::to_be(9150);
        addr_in.sin6_addr = libc::in6_addr {
            s6_addr: Ipv6Addr::LOCALHOST.octets(),
        };

        let parsed = socket_addr_from_storage(&storage, size_of::<libc::sockaddr_in6>()).unwrap();
        assert_eq!(
            parsed,
            SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 9150, 0, 0))
        );
    }
}
