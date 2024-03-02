use std::io;

use tokio::io::unix::{AsyncFd, TryIoError};
use tracing::instrument;

#[derive(Debug)]
pub struct TunInterface {
    pub inner: AsyncFd<crate::TunInterface>,
}

impl TunInterface {
    #[instrument]
    pub fn new(mut tun: crate::TunInterface) -> io::Result<Self> {
        tun.set_nonblocking(true)?;
        Ok(Self { inner: AsyncFd::new(tun)? })
    }

    #[instrument]
    pub async fn set_up(&self, up: bool) -> io::Result<()> {
        let mut guard = self.inner.readable().await?;
        guard.try_io(|inner| inner.get_ref().set_up(up));
        Ok(())
    }

    #[instrument]
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.writable().await?;
            match guard.try_io(|inner| inner.get_ref().send(buf)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|inner| inner.get_ref().recv(buf)) {
                Ok(result) => return result,
                Err(_would_block) => {
                    tracing::debug!("WouldBlock");
                    continue
                }
            }
        }
    }
}
