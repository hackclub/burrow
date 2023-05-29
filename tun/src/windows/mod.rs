use fehler::throws;
use std::io::Error;
use std::ptr;
use widestring::{u16cstr, U16CString};
use windows::Win32::Foundation::GetLastError;
mod queue;

pub use queue::TunQueue;

pub struct TunInterface {
    wintun: sys::wintun,
    handle: sys::WINTUN_ADAPTER_HANDLE,
    name: String,
}

impl TunInterface {
    #[throws]
    pub fn new() -> TunInterface {
        let name = U16CString::from(u16cstr!("Burrow"));
        let wintun = sys::wintun::default();
        let handle =
            unsafe { wintun.WintunCreateAdapter(name.as_ptr(), name.as_ptr(), ptr::null()) };
        if handle.is_null() {
            unsafe { GetLastError() }.ok()?
        }
        TunInterface {
            wintun,
            handle,
            name: String::from("Burrow"),
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl Drop for TunInterface {
    fn drop(&mut self) {
        unsafe { self.wintun.WintunCloseAdapter(self.handle) }
    }
}

pub(crate) mod sys {
    #![allow(dead_code, non_camel_case_types, non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/wintun.rs"));

    impl Default for wintun {
        fn default() -> Self {
            unsafe { wintun::new(format!("{}/wintun.dll", env!("OUT_DIR"))).unwrap() }
        }
    }
}
