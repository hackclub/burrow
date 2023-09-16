use std::{io, mem};
use crate::TunInterface;
use std::net::{IpAddr, SocketAddr};
use std::process::Command;
use fehler::throws;
// use rtnetlink;
use std::io::{Error, Write};
use libc::{rt_msghdr, c_uchar, rt_metrics, sockaddr_dl};
use std::net::{Ipv4Addr, SocketAddrV4};
use socket2::SockAddr;

struct Handle {
    pub inner: socket2::Socket,
}

impl Handle {
    #[throws]
    pub fn new() -> Self {
        let inner = socket2::Socket::new(socket2::Domain::from(libc::PF_ROUTE), socket2::Type::RAW, None)?;
        Self { inner }
    }
}

#[derive(Debug)]
pub struct Route<'a> {
    pub destination: IpAddr, // Default: 0.0.0.0
    pub destination_prefix: u8,

    pub interface: &'a TunInterface,

    // #[cfg(target_os = "linux")]
    // pub ifindex: Option<u32>,
    //
    // #[cfg(target_os = "linux")]
    // pub table: u8,
}

impl<'a> Route<'a> {
    #[throws]
    pub fn new(destination: IpAddr, destination_prefix: u8, interface: &'a TunInterface) -> Self {
        let mut handle = Handle::new()?;

        let mut hdr: rt_msghdr = unsafe { mem::zeroed() };
        hdr.rtm_version = libc::RTM_VERSION as c_uchar;
        hdr.rtm_type = libc::RTM_ADD as c_uchar;
        hdr.rtm_flags = libc::RTF_STATIC | libc::RTF_UP;
        hdr.rtm_addrs = libc::RTA_DST | libc::RTA_NETMASK | libc::RTA_GATEWAY | libc::RTA_IFP;
        hdr.rtm_seq = 1;

        let destination_addr = SockAddr::from(SocketAddr::new(destination, 0));
        let gateway_addr = SockAddr::from(SocketAddrV4::new(interface.ipv4_addr()?, 0));

        let index = interface.index()?;
        let (_, if_addr) = unsafe {
            SockAddr::init(|storage, len| {
                *len = mem::size_of::<libc::sockaddr_dl>() as u32;

                let mut addr: &mut libc::sockaddr_dl = &mut *storage.cast();
                addr.sdl_len = *len as u8;
                addr.sdl_family = libc::AF_LINK as u8;
                addr.sdl_index = index as u16;
                Ok(())
            })?
        };
        let mask_addr = SockAddr::from(SocketAddrV4::new(Ipv4Addr::new(255, 255, 255, 255), 0));

        hdr.rtm_msglen = (
            (mem::size_of::<rt_msghdr>() as u32) + destination_addr.len() + gateway_addr.len() + if_addr.len() + mask_addr.len()
        ) as u16;

        println!("0z {:#?}", if_addr.len());
        let buffer = vec![];
        let mut cursor = std::io::Cursor::new(buffer);
        cursor.write_all(unsafe {
            std::slice::from_raw_parts(&hdr as *const _ as *const _, mem::size_of::<rt_msghdr>())
        })?;

        println!("one");
        write_addr(&mut cursor, destination_addr)?;
        println!("two");
        write_addr(&mut cursor, gateway_addr)?;
        println!("three");
        write_addr(&mut cursor, if_addr)?;
        println!("4");
        write_addr(&mut cursor, mask_addr)?;
        println!("five");

        let buf = cursor.into_inner();
        println!("cbuf len: {:#?}, calcsize: {:#?}", buf.len(), hdr.rtm_msglen);

        handle.inner.write_all(&buf)?;

        // Create handle
        Self {
            destination,
            destination_prefix,
            interface
        }
    }

    #[cfg(target_os = "linux")]
    pub fn through_interface(&self, iface: TunInterface) {
        // High-level overview:
        // 1. Create a socket
        iface.socket.bind(iface.addr);

        // 2. Bind the socket to the interface
        // 3. Add the route
        // 4. Close the socket
        // 5. Return the result
    }
}

#[throws]
#[cfg(target_platform = "linux")]
pub async fn add_route(route: Route) {
    let conn = rtnetlink::new_connection()?;

    let handle = conn.route();
    let route_add_request = handle
        .add()
        .v4()
        .destination_prefix(route.destination, route.destination_prefix)
        .output_interface(route.interface.index()?)
        .execute().await?;
}

#[throws]
#[cfg(target_vendor="apple")]
pub fn add_route(route: Route) {
    // Construct
    // Construct RT message
    Command::new("route")
        .arg("add")
        .arg("-host")
        .arg(route.destination.to_string())
        .arg("-interface")
        .arg(route.interface.name().expect("the interface's name"))
        .spawn()
        .expect("failed to execute add route process");
}

#[throws]
fn write_addr<T: Write>(sock: &mut T, addr: socket2::SockAddr) {
    let len = addr.len() as usize;
    let ptr = addr.as_ptr() as *const _;
    sock.write_all(unsafe { std::slice::from_raw_parts(ptr, len) })?;
}