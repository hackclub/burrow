use std::io;
use tokio::io::unix::AsyncFd;
use tracing::instrument;

#[derive(Debug)]
pub struct TunInterface {
    inner: AsyncFd<crate::TunInterface>,
}

impl TunInterface {
    #[instrument]
    pub fn new(tun: crate::TunInterface) -> io::Result<Self> {
        Ok(Self {
            inner: AsyncFd::new(tun)?,
        })
    }

    #[instrument]
    pub async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.writable().await?;
            match guard.try_io(|inner| inner.get_ref().send(buf)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    #[instrument]
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
