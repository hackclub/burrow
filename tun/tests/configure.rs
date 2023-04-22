use fehler::throws;
use tun::TunInterface;
use std::io::Error;
use std::net::Ipv4Addr;

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