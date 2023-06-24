use fehler::throws;
use std::io::Error;
use std::ptr;
use widestring::{u16cstr, U16CString};
use windows::Win32::Foundation::GetLastError;
mod queue;

use super::TunInterfaceOptions;

pub use queue::TunQueue;

pub struct TunInterface {
    handle: sys::WINTUN_ADAPTER_HANDLE,
    name: String,
}

impl TunInterface {
    #[throws]
    pub fn new() -> TunInterface {
        Self::new_with_options(TunInterfaceOptions::new())?
    }

    #[throws]
    pub(crate) fn new_with_options(options: TunInterfaceOptions) -> TunInterface {
        let name_owned = options.name.unwrap_or("Burrow".to_owned());
        let name = U16CString::from_str(&name_owned).unwrap();

        let handle =
            unsafe { sys::WINTUN.WintunCreateAdapter(name.as_ptr(), name.as_ptr(), ptr::null()) };
        if handle.is_null() {
            unsafe { GetLastError() }.ok()?
        }
        TunInterface {
            handle,
            name: name_owned,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl Drop for TunInterface {
    fn drop(&mut self) {
        unsafe { sys::WINTUN.WintunCloseAdapter(self.handle) }
    }
}

pub(crate) mod sys {
    #![allow(clippy::all, dead_code, non_camel_case_types, non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/wintun.rs"));

    const WINTUN_BINARY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wintun.dll"));

    lazy_static::lazy_static! {
        pub static ref WINTUN: wintun = {
            use std::io::Write;

            let mut temp_file = tempfile::NamedTempFile::new().unwrap();
            temp_file.write_all(&WINTUN_BINARY).unwrap();
            let (_, path) = temp_file.keep().unwrap();

            unsafe { wintun::new(&path) }.unwrap()
        };
    }
}
