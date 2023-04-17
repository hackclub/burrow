use fehler::throws;
use std::{
    io::{self, Error},
    mem::MaybeUninit,
};
use tokio::io::unix::AsyncFd;

pub struct TunQueue {
    io: AsyncFd<tun::TunQueue>,
}

impl TunQueue {
    #[throws]
    pub fn from_queue(queue: tun::TunQueue) -> Self {
        Self {
            io: AsyncFd::new(queue)?,
        }
    }

    pub async fn try_recv(&self, buf: &mut [MaybeUninit<u8>]) -> io::Result<usize> {
        loop {
            let mut guard = self.io.readable().await?;
            match guard.try_io(|inner| inner.get_ref().recv(buf)) {
                Ok(result) => return result,
                Err(..) => continue,
            }
        }
    }
}
