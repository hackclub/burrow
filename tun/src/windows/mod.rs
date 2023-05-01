use std::io::Result;
use std::ptr;
use widestring::{u16cstr, U16CString};

mod queue;

pub use queue::TunQueue;

pub struct TunInterface {
    wintun: sys::wintun,
    handle: sys::WINTUN_ADAPTER_HANDLE,
    name: String,
}

impl TunInterface {
    pub fn new() -> Result<TunInterface> {
        let name = U16CString::from(u16cstr!("ConradNet"));
        let wintun = sys::wintun::default();
        let handle =
            unsafe { wintun.WintunCreateAdapter(name.as_ptr(), name.as_ptr(), ptr::null()) };
        Ok(TunInterface {
            wintun,
            handle,
            name: String::from("ConradNet"),
        })
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[throws]
    pub async fn reroute(&mut self, interface_addr: Ipv4Addr, dest: Ipv4Addr, gateway: Ipv4Addr) {
        todo!()
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
