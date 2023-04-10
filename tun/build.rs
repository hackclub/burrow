#[cfg(windows)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    use std::io::{Cursor, Read};

    let buf = reqwest::get("https://www.wintun.net/builds/wintun-0.14.1.zip")
        .await?
        .bytes()
        .await?;
    assert_content_hash(
        &buf,
        hex_literal::hex!("07c256185d6ee3652e09fa55c0b673e2624b565e02c4b9091c79ca7d2f24ef51"),
    );
    let mut archive = zip::ZipArchive::new(Cursor::new(buf))?;

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);

    let mut header = String::new();
    archive
        .by_name("wintun/include/wintun.h")?
        .read_to_string(&mut header)?;
    header.push_str(
        "WINTUN_CLOSE_ADAPTER_FUNC WintunCloseAdapter;
        WINTUN_OPEN_ADAPTER_FUNC WintunOpenAdapter;
        WINTUN_GET_ADAPTER_LUID_FUNC WintunGetAdapterLUID;
        WINTUN_GET_RUNNING_DRIVER_VERSION_FUNC WintunGetRunningDriverVersion;
        WINTUN_DELETE_DRIVER_FUNC WintunDeleteDriver;
        WINTUN_SET_LOGGER_FUNC WintunSetLogger;
        WINTUN_START_SESSION_FUNC WintunStartSession;
        WINTUN_END_SESSION_FUNC WintunEndSession;
        WINTUN_CREATE_ADAPTER_FUNC WintunCreateAdapter;
        WINTUN_GET_READ_WAIT_EVENT_FUNC WintunGetReadWaitEvent;
        WINTUN_RECEIVE_PACKET_FUNC WintunReceivePacket;
        WINTUN_RELEASE_RECEIVE_PACKET_FUNC WintunReleaseReceivePacket;
        WINTUN_ALLOCATE_SEND_PACKET_FUNC WintunAllocateSendPacket;
        WINTUN_SEND_PACKET_FUNC WintunSendPacket;",
    );
    let bindings = bindgen::Builder::default()
        .header_contents("wintun.h", &header)
        .allowlist_function("Wintun.*")
        .allowlist_type("WINTUN_.*")
        .dynamic_library_name("wintun")
        .dynamic_link_require_all(true)
        .generate()
        .unwrap();
    bindings.write_to_file(out_dir.join("wintun.rs"))?;

    let mut library = Vec::new();
    let platform = platforms::Platform::find(&env::var("TARGET")?).unwrap();
    let arch = match platform.target_arch {
        platforms::target::Arch::Arm => "arm",
        platforms::Arch::AArch64 => "arm64",
        platforms::Arch::X86 => "x86",
        platforms::Arch::X86_64 => "amd64",
        arch => panic!("{} is not a supported architecture", arch),
    };
    archive
        .by_name(&format!("wintun/bin/{}/wintun.dll", arch))?
        .read_to_end(&mut library)?;
    std::fs::write(out_dir.join("wintun.dll"), library)?;

    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

#[cfg(not(windows))]
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}

#[cfg(windows)]
fn assert_content_hash(content: &[u8], hash: [u8; 32]) {
    use sha2::digest::Update;
    use sha2::Digest;

    let computed = sha2::Sha256::new().chain(content).finalize();
    assert_eq!(computed.as_slice(), &hash[..]);
}
