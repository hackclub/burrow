use super::*;
use anyhow::anyhow;
use log::log;
use std::hash::Hash;
use std::path::PathBuf;
use std::{
    ascii, io,
    os::fd::{FromRawFd, RawFd},
    os::unix::net::UnixListener as StdUnixListener,
    path::Path,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};
use tracing::debug;
use tracing::info;

#[cfg(not(target_vendor = "apple"))]
const UNIX_SOCKET_PATH: &str = "/run/burrow.sock";

#[cfg(target_vendor = "apple")]
const UNIX_SOCKET_PATH: &str = "burrow.sock";

#[cfg(target_os = "macos")]
fn fetch_socket_path() -> Option<PathBuf> {
    let tries = vec![
        "burrow.sock".to_string(),
        format!(
            "{}/Library/Containers/com.hackclub.burrow.network/Data/burrow.sock",
            std::env::var("HOME").unwrap_or_default()
        )
        .to_string(),
    ];
    for path in tries {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
fn fetch_socket_path() -> Option<PathBuf> {
    Some(Path::new(UNIX_SOCKET_PATH).to_path_buf())
}

pub async fn listen(
    cmd_tx: async_channel::Sender<DaemonCommand>,
    rsp_rx: async_channel::Receiver<DaemonResponse>,
) -> Result<()> {
    listen_with_optional_fd(cmd_tx, rsp_rx, None).await
}

pub(crate) async fn listen_with_optional_fd(
    cmd_tx: async_channel::Sender<DaemonCommand>,
    rsp_rx: async_channel::Receiver<DaemonResponse>,
    raw_fd: Option<RawFd>,
) -> Result<()> {
    let path = Path::new(UNIX_SOCKET_PATH);

    let listener = if let Some(raw_fd) = raw_fd {
        let listener = unsafe { StdUnixListener::from_raw_fd(raw_fd) };
        listener.set_nonblocking(true)?;
        UnixListener::from_std(listener)
    } else {
        UnixListener::bind(path)
    };
    let listener = if let Ok(listener) = listener {
        listener
    } else {
        //  Won't help all that much, if we use the async version of fs.
        if let Some(par) = path.parent() {
            std::fs::create_dir_all(par)?;
        }
        match std::fs::remove_file(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            stuff => stuff,
        }?;
        info!("Relative path: {}", path.to_string_lossy());
        UnixListener::bind(path)?
    };
    loop {
        let (stream, _) = listener.accept().await?;
        let cmd_tx = cmd_tx.clone();

        //  I'm pretty sure we won't need to manually join / shut this down,
        //  `lines` will return Err during dropping, and this task should exit gracefully.
        let rsp_rxc = rsp_rx.clone();
        tokio::task::spawn(async move {
            let cmd_tx = cmd_tx;
            let mut stream = stream;
            let (mut read_stream, mut write_stream) = stream.split();
            let buf_reader = BufReader::new(&mut read_stream);
            let mut lines = buf_reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                info!("Got line: {}", line);
                debug!("Line raw data: {:?}", line.as_bytes());
                let mut res: DaemonResponse = DaemonResponseData::None.into();
                let req = match serde_json::from_str::<DaemonRequest>(&line) {
                    Ok(req) => Some(req),
                    Err(e) => {
                        res.result = Err(e.to_string());
                        None
                    }
                };
                let mut res = serde_json::to_string(&res).unwrap();
                res.push('\n');

                if let Some(req) = req {
                    cmd_tx.send(req.command).await.unwrap();
                    let res = rsp_rxc.recv().await.unwrap().with_id(req.id);
                    let mut retres = serde_json::to_string(&res).unwrap();
                    retres.push('\n');
                    info!("Sending response: {}", retres);
                    write_stream.write_all(retres.as_bytes()).await.unwrap();
                }
            }
        });
    }
}

#[derive(Debug)]
pub struct DaemonClient {
    connection: UnixStream,
}

impl DaemonClient {
    pub async fn new() -> Result<Self> {
        let path = fetch_socket_path().ok_or(anyhow!("Failed to find socket path"))?;
        // debug!("found path: {:?}", path);
        let connection = UnixStream::connect(path).await?;
        debug!("connected to socket");
        Ok(Self { connection })
    }

    pub async fn new_with_path(path: &str) -> Result<Self> {
        let path = Path::new(path);
        let connection = UnixStream::connect(path).await?;

        Ok(Self { connection })
    }

    pub async fn send_command(&mut self, command: DaemonCommand) -> Result<DaemonResponse> {
        let mut command = serde_json::to_string(&DaemonRequest { id: 0, command })?;
        command.push('\n');

        self.connection.write_all(command.as_bytes()).await?;
        let buf_reader = BufReader::new(&mut self.connection);
        let mut lines = buf_reader.lines();
        let response = lines
            .next_line()
            .await?
            .ok_or(anyhow!("Failed to read response"))?;
        debug!("Got raw response: {}", response);
        let res: DaemonResponse = serde_json::from_str(&response)?;
        Ok(res)
    }
}
