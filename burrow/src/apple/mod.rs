use std::net::Ipv4Addr;
use std::io::Error;
use std::os::fd::FromRawFd;
use tracing::field::debug;
use tun::TunInterface;
use tracing_oslog::OsLogger;
use tracing_subscriber::layer::SubscriberExt;
use tracing::debug;

#[repr(C)]
pub struct NetWorkSettings {
    ipv4_addr: i64,
    ipv4_netmask: i64,
    mtu: i32,
}

fn encode_ipv4_result(res: Result<Ipv4Addr, Error>) -> i64 {
    match res {
        Ok(addr) => u32::from_be(addr.into()) as i64,
        Err(_) => -1,
    }
}

impl From<TunInterface> for NetWorkSettings {
    fn from(value: TunInterface) -> Self {
        debug!("Converting TunInterface {} to NetWorkSettings", value.name().unwrap_or("NONEXISTENT".to_string()));
        let ipv4_addr = encode_ipv4_result(value.ipv4_addr());
        let ipv4_netmask = encode_ipv4_result(value.netmask());
        let mtu = value.mtu().unwrap_or(-1);
        Self {
            ipv4_addr,
            ipv4_netmask,
            mtu,
        }
    }
}

#[no_mangle]
pub extern "C" fn initialize_oslog() {
    let collector = tracing_subscriber::registry()
        .with(OsLogger::new("com.hackclub.burrow", "default"));
    tracing::subscriber::set_global_default(collector).unwrap();
    debug!("Initialized oslog tracing in libburrow rust FFI");
}

#[no_mangle]
pub extern "C" fn getNetworkSettings(n: i32) -> NetWorkSettings {
    debug!("getNetworkSettings called with fd: {}", n);
    let iface = unsafe {TunInterface::from_raw_fd(n)};
    iface.into()
}