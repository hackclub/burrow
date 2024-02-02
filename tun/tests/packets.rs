use std::{io::Error, net::Ipv4Addr};
use std::net::Ipv6Addr;

use fehler::throws;
use tun::TunInterface;

#[throws]
#[test]
#[ignore = "requires interactivity"]
#[cfg(not(target_os = "windows"))]
fn tst_read() {
    // This test is interactive, you need to send a packet to any server through
    // 192.168.1.10 EG. `sudo route add 8.8.8.8 192.168.1.10`,
    //`dig @8.8.8.8 hackclub.com`
    let tun = TunInterface::new()?;
    println!("tun name: {:?}", tun.name()?);
    tun.set_ipv4_addr(Ipv4Addr::from([192, 168, 1, 10]))?;
    println!("tun ip: {:?}", tun.ipv4_addr()?);
    println!("Waiting for a packet...");
    let buf = &mut [0u8; 1500];
    let res = tun.recv(buf);
    println!("Received!");
    assert!(res.is_ok());
}

#[test]
#[throws]
#[ignore = "requires interactivity"]
#[cfg(not(target_os = "windows"))]
fn write_packets() {
    let tun = TunInterface::new()?;
    let mut buf = [0u8; 1500];
    buf[0] = 6 << 4;
    let bytes_written = tun.send(&buf)?;
    assert_eq!(bytes_written, 1504);
}

#[test]
#[throws]
#[ignore = "requires interactivity"]
#[cfg(not(target_os = "windows"))]
fn set_ipv6() {
    let tun = TunInterface::new()?;
    println!("tun name: {:?}", tun.name()?);
    let targ_addr: Ipv6Addr = "::1".parse().unwrap();
    println!("v6 addr: {:?}", targ_addr);
    tun.set_ipv6_addr(targ_addr)?;
}