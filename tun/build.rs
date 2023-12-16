#[cfg(not(windows))]
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}

#[cfg(windows)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let out_dir = std::env::var("OUT_DIR")?;
    let path = std::path::PathBuf::from(out_dir);
    generate(&path).await?;

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

#[cfg(windows)]
async fn generate(out_dir: &std::path::Path) -> anyhow::Result<()> {
    use std::{fs::File, io::Write};

    use anyhow::Context;

    let bindings_path = out_dir.join("wintun.rs");
    let binary_path = out_dir.join("wintun.dll");
    println!("cargo:rerun-if-changed={}", bindings_path.to_str().unwrap());
    println!("cargo:rerun-if-changed={}", binary_path.to_str().unwrap());

    if let (Ok(..), Ok(..)) = (File::open(&bindings_path), File::open(&binary_path)) {
        return Ok(())
    };

    let archive = download(out_dir)
        .await
        .context("Failed to download wintun")?;

    let (bindings, binary) = parse(archive).context("Failed to parse wintun archive")?;

    bindings
        .write_to_file(bindings_path)
        .context("Failed to write bindings")?;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(binary_path)
        .context("Failed to write binary")?;
    file.write_all(&binary)?;

    Ok(())
}

#[cfg(windows)]
async fn download(directory: &std::path::Path) -> anyhow::Result<std::fs::File> {
    use std::{io::Write, str::FromStr};

    let path = directory.join(WINTUN_FILENAME);
    let mut file = match std::fs::OpenOptions::new().read(true).open(&path) {
        Ok(existing) => return Ok(existing),
        Err(_e) => std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?,
    };

    let mut url = reqwest::Url::from_str("https://www.wintun.net/builds")?;
    url.path_segments_mut().unwrap().push(WINTUN_FILENAME);

    let body = reqwest::get(url).await?.bytes().await?;

    ssri::IntegrityChecker::new(WINTUN_INTEGRITY.parse()?)
        .chain(&body)
        .result()?;

    file.set_len(0)?;
    file.write_all(&body)?;

    Ok(file)
}

#[cfg(windows)]
fn parse(file: std::fs::File) -> anyhow::Result<(bindgen::Bindings, Vec<u8>)> {
    use std::io::Read;

    use anyhow::Context;

    let reader = std::io::BufReader::new(file);
    let mut archive = zip::ZipArchive::new(reader)?;

    let mut header = String::new();
    archive
        .by_name("wintun/include/wintun.h")?
        .read_to_string(&mut header)?;
    header.push_str(WINTUN_BINDINGS_PREAMBLE);

    let bindings = bindgen::Builder::default()
        .header_contents("wintun.h", &header)
        .allowlist_function("Wintun.*")
        .allowlist_type("WINTUN_.*")
        .dynamic_library_name("wintun")
        .dynamic_link_require_all(true)
        .generate()
        .context("Failed to generate bindings from wintun archive")
        .unwrap();

    let mut binary = Vec::new();
    let target = std::env::var("TARGET")?;
    let arch = match target.split('-').next() {
        Some("i686") => "x86",
        Some("x86_64") => "amd64",
        Some("aarch64") => "arm64",
        Some("thumbv7a") => "arm",
        Some(a) => panic!("{} is not a supported architecture", a),
        None => unreachable!(),
    };
    archive
        .by_name(&format!("wintun/bin/{}/wintun.dll", arch))?
        .read_to_end(&mut binary)
        .context("Failed to read binary from wintun archive")?;

    Ok((bindings, binary))
}

#[cfg(windows)]
const WINTUN_FILENAME: &str = "wintun-0.14.1.zip";

#[cfg(windows)]
const WINTUN_INTEGRITY: &str = "sha256-B8JWGF1u42UuCfpVwLZz4mJLVl4CxLkJHHnKfS8k71E=";

#[cfg(windows)]
const WINTUN_BINDINGS_PREAMBLE: &str = r#"
WINTUN_CLOSE_ADAPTER_FUNC WintunCloseAdapter;
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
WINTUN_SEND_PACKET_FUNC WintunSendPacket;
"#;
