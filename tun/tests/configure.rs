use std::{io::Error, net::Ipv4Addr};

use fehler::throws;
use tun::TunInterface;

fn open_tun() -> Result<Option<TunInterface>, Error> {
    match TunInterface::new() {
        Ok(tun) => Ok(Some(tun)),
        Err(err)
            if err.kind() == std::io::ErrorKind::PermissionDenied
                || matches!(err.raw_os_error(), Some(1 | 13)) =>
        {
            eprintln!("skipping tun test without tunnel privileges: {err}");
            Ok(None)
        }
        Err(err) => Err(err),
    }
}

#[test]
#[throws]
fn test_create() {
    let _ = open_tun()?;
}

#[test]
#[throws]
#[cfg(not(any(target_os = "windows", target_vendor = "apple")))]
fn test_set_get_broadcast_addr() {
    let Some(tun) = open_tun()? else {
        return Ok(());
    };
    let addr = Ipv4Addr::new(10, 0, 0, 1);
    tun.set_ipv4_addr(addr)?;

    let broadcast_addr = Ipv4Addr::new(255, 255, 255, 0);
    tun.set_broadcast_addr(broadcast_addr)?;
    let result = tun.broadcast_addr()?;

    assert_eq!(broadcast_addr, result);
}

#[test]
#[throws]
#[cfg(not(target_os = "windows"))]
fn test_set_get_ipv4() {
    let Some(tun) = open_tun()? else {
        return Ok(());
    };

    let addr = Ipv4Addr::new(10, 0, 0, 1);
    tun.set_ipv4_addr(addr)?;
    let result = tun.ipv4_addr()?;

    assert_eq!(addr, result);
}

#[test]
#[throws]
#[cfg(not(any(target_os = "windows", target_vendor = "apple")))]
fn test_set_get_ipv6() {
    use std::net::Ipv6Addr;

    let Some(tun) = open_tun()? else {
        return Ok(());
    };

    let addr = Ipv6Addr::new(1, 1, 1, 1, 1, 1, 1, 1);
    tun.add_ipv6_addr(addr, 128)?;

    // let result = tun.ipv6_addr()?;
    // assert_eq!(addr, result);
}

#[test]
#[throws]
#[cfg(not(target_os = "windows"))]
fn test_set_get_mtu() {
    let Some(interf) = open_tun()? else {
        return Ok(());
    };

    interf.set_mtu(500)?;

    assert_eq!(interf.mtu().unwrap(), 500);
}

#[test]
#[throws]
#[cfg(not(target_os = "windows"))]
fn test_set_get_netmask() {
    let Some(interf) = open_tun()? else {
        return Ok(());
    };

    let netmask = Ipv4Addr::new(255, 0, 0, 0);
    let addr = Ipv4Addr::new(192, 168, 1, 1);

    interf.set_ipv4_addr(addr)?;
    interf.set_netmask(netmask)?;

    assert_eq!(interf.netmask()?, netmask);
}
