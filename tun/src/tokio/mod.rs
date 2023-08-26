use std::io;
use tokio::io::unix::AsyncFd;

pub struct TunInterface {
    inner: AsyncFd<crate::TunInterface>,
}

impl TunInterface {
    pub fn new(tun: crate::TunInterface) -> io::Result<Self> {
        Ok(Self {
            inner: AsyncFd::new(tun)?,
        })
    }

    pub async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.writable().await?;
            match guard.try_io(|inner| inner.get_ref().send(buf)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.readable_mut().await?;
            match guard.try_io(|inner| (*inner).get_mut().recv(buf)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn mtu(&self) -> io::Result<i32> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().mtu()) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn set_mtu(&self, mtu: i32) -> io::Result<()> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().set_mtu(mtu)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn name(&self) -> io::Result<String> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().name()) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn set_name(&self, name: &str) -> io::Result<()> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().set_name(name)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn netmask(&self) -> io::Result<std::net::Ipv4Addr> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().netmask()) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn set_netmask(&self, netmask: std::net::Ipv4Addr) -> io::Result<()> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().set_netmask(netmask)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn ipv4_addr(&self) -> io::Result<std::net::Ipv4Addr> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().ipv4_addr()) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn set_ipv4_addr(&self, addr: std::net::Ipv4Addr) -> io::Result<()> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().set_ipv4_addr(addr)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }
}
