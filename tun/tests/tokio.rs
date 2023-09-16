use std::net::Ipv4Addr;

#[tokio::test]
#[cfg(all(feature = "tokio", not(target_os = "windows")))]
async fn test_create() {
    let tun = tun::TunInterface::new().unwrap();
    let _ = tun::tokio::TunInterface::new(tun).unwrap();
}

#[tokio::test]
#[ignore = "requires interactivity"]
#[cfg(all(feature = "tokio", not(target_os = "windows")))]
async fn test_write() {
    let tun = tun::TunInterface::new().unwrap();
    tun.set_ipv4_addr(Ipv4Addr::from([192, 168, 1, 10]))
        .unwrap();
    let async_tun = tun::tokio::TunInterface::new(tun).unwrap();
    let mut buf = [0u8; 1500];
    buf[0] = 6 << 4;
    let bytes_written = async_tun.send(&buf).await.unwrap();
    assert!(bytes_written > 0);
}
