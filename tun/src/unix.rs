use std::ffi::{c_char, CStr};

pub fn copy_if_name(buf: [c_char; libc::IFNAMSIZ]) -> String {
    // TODO: Switch to `CStr::from_bytes_until_nul` when stabilized
    unsafe {
        CStr::from_ptr(buf.as_ptr() as *const _)
            .to_str()
            .unwrap()
            .to_string()
    }
}
