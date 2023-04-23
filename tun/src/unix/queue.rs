use fehler::throws;

use std::{
    io::{Error, Read, Write},
    mem::MaybeUninit,
    os::unix::io::{AsRawFd, IntoRawFd, RawFd},
};

use crate::TunInterface;

pub struct TunQueue {
    socket: socket2::Socket,
}

impl TunQueue {
    #[throws]
    pub fn recv(&self, buf: &mut [MaybeUninit<u8>]) -> usize {
        self.socket.recv(buf)?
    }
}

impl Read for TunQueue {
    #[throws]
    fn read(&mut self, buf: &mut [u8]) -> usize {
        self.socket.read(buf)?
    }
}

impl Write for TunQueue {
    #[throws]
    fn write(&mut self, buf: &[u8]) -> usize {
        self.socket.write(buf)?
    }

    #[throws]
    fn flush(&mut self) {
        self.socket.flush()?
    }
}

impl TunQueue {
    pub fn from<T: TunInterface>(interface: T) -> TunQueue {
        TunQueue {
            socket: interface.into_raw_socket(),
        }
    }
}

impl AsRawFd for TunQueue {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

//TODO: error[E0507]: cannot move out of `*self.socket` which is behind a mutable reference

//impl IntoRawFd for TunQueue {
//    fn into_raw_fd(self) -> RawFd {
//        self.socket.into_raw_fd()
//    }
//}
