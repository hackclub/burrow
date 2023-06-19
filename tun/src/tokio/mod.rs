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
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    #[tokio::test]
    async fn test_create() {
        let tun = crate::TunInterface::new().unwrap();
        let _async_tun = TunInterface::new(tun).unwrap();
    }

    #[tokio::test]
    async fn test_write() {
        let tun = crate::TunInterface::new().unwrap();
        tun.set_ipv4_addr(Ipv4Addr::from([192, 168, 1, 10]))
            .unwrap();
        let async_tun = TunInterface::new(tun).unwrap();
        let mut buf = [0u8; 1500];
        buf[0] = 6 << 4;
        let bytes_written = async_tun.write(&buf).await.unwrap();
        assert!(bytes_written > 0);
    }
}
