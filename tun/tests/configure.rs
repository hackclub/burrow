use fehler::throws;
use std::io::Error;
use std::net::{Ipv4Addr, Ipv6Addr};
use tun::TunInterface;

#[test]
#[throws]
fn test_create() {
    TunInterface::new()?;
}

#[test]
#[throws]
fn test_set_get_ipv4() {
    let tun = TunInterface::new()?;

    let addr = Ipv4Addr::new(10, 0, 0, 1);
    tun.set_ipv4_addr(addr)?;
    let result = tun.ipv4_addr()?;

    assert_eq!(addr, result);
}

#[test]
#[throws]
fn test_set_get_ipv6() {
    let tun = TunInterface::new()?;

    let addr = Ipv6Addr::new(1, 1, 1, 1, 1, 1, 1, 1);
    tun.set_ipv6_addr(addr)?;

    // let result = tun.ipv6_addr()?;
    // assert_eq!(addr, result);
}
