use burrow::log;
use rtnetlink::{new_connection, Error};
use std::net::Ipv4Addr;
use tun::TunInterface;

pub async fn reroute(
    interface: TunInterface,
    interface_addr: Ipv4Addr,
    dest: Ipv4Addr,
    gateway: Ipv4Addr,
) -> ! {
    let name = interface.name().expect("a valid interface name");
    let (connection, handle, _) = new_connection().expect("a new netlink connection");
    let netlink_connection_handle = tokio::task::spawn(connection);

    interface.set_iface_up().expect("an active interface");
    log(format!("Interface {name} is up"));

    interface
        .set_ipv4_addr(interface_addr)
        .expect("to set the interface's IPv4 address");
    log("Interface IPv4 address set");

    _ = handle.link().get().match_name(name).execute();
    log("Interface link retrieved");

    let route_result = handle
        .route()
        .add()
        .v4()
        .destination_prefix(dest, 0)
        .gateway(gateway)
        .execute()
        .await;

    if let Err(Error::NetlinkError(err)) = &route_result {
        if err.code == -19 {
            panic!("the route already exists");
        }
    } else {
        log("Route added successfully!");
    }

    netlink_connection_handle.abort();

    #[allow(clippy::empty_loop)]
    loop {}
}
