#[cfg(all(feature = "tokio", not(target_os = "windows")))]
use std::net::Ipv4Addr;

#[cfg(all(feature = "tokio", not(target_os = "windows")))]
fn open_tun() -> Option<tun::TunInterface> {
    match tun::TunInterface::new() {
        Ok(tun) => Some(tun),
        Err(err)
            if err.kind() == std::io::ErrorKind::PermissionDenied
                || matches!(err.raw_os_error(), Some(1 | 13)) =>
        {
            eprintln!("skipping tokio tun test without tunnel privileges: {err}");
            None
        }
        Err(err) => panic!("failed to create tun interface: {err}"),
    }
}

#[tokio::test]
#[cfg(all(feature = "tokio", not(target_os = "windows")))]
async fn test_create() {
    let Some(tun) = open_tun() else {
        return;
    };
    let _ = tun::tokio::TunInterface::new(tun).unwrap();
}

#[tokio::test]
#[ignore = "requires interactivity"]
#[cfg(all(feature = "tokio", not(target_os = "windows")))]
async fn test_write() {
    let Some(tun) = open_tun() else {
        return;
    };
    tun.set_ipv4_addr(Ipv4Addr::from([192, 168, 1, 10]))
        .unwrap();
    let async_tun = tun::tokio::TunInterface::new(tun).unwrap();
    let mut buf = [0u8; 1500];
    buf[0] = 6 << 4;
    let bytes_written = async_tun.send(&buf).await.unwrap();
    assert!(bytes_written > 0);
}
