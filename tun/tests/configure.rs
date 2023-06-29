use fehler::throws;
use std::io::Error;
use std::net::{Ipv4Addr};
use tun::TunInterface;

#[test]
#[throws]
fn test_create() {
    TunInterface::new()?;
}

#[test]
#[throws]
#[cfg(not(target_os = "windows"))]
fn test_set_get_ipv4() {
    let tun = TunInterface::new()?;

    let addr = Ipv4Addr::new(10, 0, 0, 1);
    tun.set_ipv4_addr(addr)?;
    let result = tun.ipv4_addr()?;

    assert_eq!(addr, result);
}

#[test]
#[throws]
#[cfg(not(any(target_os = "windows", target_vendor = "apple")))]
fn test_set_get_ipv6() {
    let tun = TunInterface::new()?;

    let addr = Ipv6Addr::new(1, 1, 1, 1, 1, 1, 1, 1);
    tun.set_ipv6_addr(addr)?;

    // let result = tun.ipv6_addr()?;
    // assert_eq!(addr, result);
}

#[test]
#[throws]
#[cfg(not(target_os = "windows"))]
fn test_set_get_mtu() {
    let interf = TunInterface::new()?;

    interf.set_mtu(500)?;

    assert_eq!(interf.mtu().unwrap(), 500);
}

#[test]
#[throws]
#[cfg(not(target_os = "windows"))]
fn test_set_get_netmask() {
    let interf = TunInterface::new()?;

    let netmask = Ipv4Addr::new(255, 0, 0, 0);
    let addr = Ipv4Addr::new(192, 168, 1, 1);

    interf.set_ipv4_addr(addr)?;
    interf.set_netmask(netmask)?;

    assert_eq!(interf.netmask()?, netmask);
}
