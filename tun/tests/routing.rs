use fehler::throws;
use std::io::Error;
use std::net::{IpAddr, Ipv4Addr};
use tun::routing::{Route, add_route};
use tun::TunInterface;

#[test]
#[throws]
fn test_create() -> std::io::Result<()> {
    let tun = TunInterface::new()?;
    let name = tun.name()?;
    println!("Interface name: {name}");

    let addr = Ipv4Addr::new(10, 0, 0, 1);
    tun.set_ipv4_addr(addr)?;

    let dest = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));

    let route = Route::new(dest, 24, &tun)?;
    add_route(route)?;

    loop {}

    Ok(())
}
