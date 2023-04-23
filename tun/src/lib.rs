#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod imp;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[path = "unix/mod.rs"]
pub(crate) mod imp;

/**
 * Standard platform-independent interface for a tunnel.
 */
pub trait TunInterface {
    /**
     * Sets the interface IP address. Accepts either IPv6 or IPv4
     */
    fn set_ip(&self, ip: IpAddr) -> Result<(), std::io::Error>;

    /**
     * Sets the interface IP address to an IPv4 address.
     *
     * Used by [set_ip](TunInterface::set_ip)
     */
    fn set_ipv4(&self, ip: Ipv4Addr) -> Result<(), std::io::Error>;

    fn get_ip(&self) -> Result<IpAddr, std::io::Error>;

    fn get_interface_name(&self) -> Result<String, std::io::Error>;

    fn into_raw_socket(self) -> socket2::Socket;
}

#[cfg(target_os = "linux")]
pub fn create_interface() -> impl TunInterface {
    PlatformTun::new().unwrap()
}

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Mutex,
};

pub use imp::{PlatformTun, TunQueue};
