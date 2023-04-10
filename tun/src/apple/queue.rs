use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

pub struct TunQueue {
    socket: socket2::Socket,
}

impl AsRawFd for TunQueue {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl IntoRawFd for TunQueue {
    fn into_raw_fd(self) -> RawFd {
        self.socket.into_raw_fd()
    }
}
