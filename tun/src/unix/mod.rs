use std::{
    io::Error,
    mem::MaybeUninit,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
};

use tracing::instrument;

mod address;
mod queue;

#[cfg(target_vendor = "apple")]
#[path = "apple/mod.rs"]
mod imp;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod imp;

use fehler::throws;
pub use imp::TunInterface;
pub use queue::TunQueue;

impl AsRawFd for TunInterface {
    fn as_raw_fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }
}

impl FromRawFd for TunInterface {
    unsafe fn from_raw_fd(fd: RawFd) -> TunInterface {
        let socket = socket2::Socket::from_raw_fd(fd);
        TunInterface { socket }
    }
}

impl IntoRawFd for TunInterface {
    fn into_raw_fd(self) -> RawFd {
        self.socket.into_raw_fd()
    }
}

unsafe fn assume_init(buf: &[MaybeUninit<u8>]) -> &[u8] {
    &*(buf as *const [MaybeUninit<u8>] as *const [u8])
}

impl TunInterface {
    #[throws]
    #[instrument]
    pub fn recv(&self, buf: &mut [u8]) -> usize {
        let packet_information_size = self.packet_information_size();
        let mut tmp_buf = [MaybeUninit::uninit(); 1504];
        let len = unsafe {
            libc::read(
                self.as_raw_fd(),
                tmp_buf.as_mut_ptr().cast::<libc::c_void>(),
                tmp_buf.len(),
            )
        };
        if len < 0 {
            Err(Error::last_os_error())?;
        }
        let len = len as usize;
        if len < packet_information_size {
            return 0;
        }

        let result_buf = unsafe { assume_init(&tmp_buf[packet_information_size..len]) };
        buf[..len - packet_information_size].copy_from_slice(result_buf);
        len - packet_information_size
    }

    #[throws]
    #[instrument]
    pub fn set_nonblocking(&mut self, nb: bool) {
        self.socket.set_nonblocking(nb)?;
    }
}

#[instrument]
pub fn ifname_to_string(buf: [libc::c_char; libc::IFNAMSIZ]) -> String {
    // TODO: Switch to `CStr::from_bytes_until_nul` when stabilized
    unsafe {
        std::ffi::CStr::from_ptr(buf.as_ptr() as *const _)
            .to_str()
            .unwrap()
            .to_string()
    }
}

#[instrument]
pub fn string_to_ifname(name: &str) -> [libc::c_char; libc::IFNAMSIZ] {
    let mut buf = [0 as libc::c_char; libc::IFNAMSIZ];
    let len = name.len().min(buf.len());
    buf[..len].copy_from_slice(unsafe { &*(name.as_bytes() as *const _ as *const [libc::c_char]) });
    buf
}
