#[cfg(target_os = "linux")]
use std::os::fd::{IntoRawFd, RawFd};
use std::{ffi::OsStr, io, path::Path};

use anyhow::{anyhow, Error, Result};
use fehler::throws;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};
use tracing::{debug, error, info};

use crate::daemon::rpc::{
    DaemonCommand,
    DaemonMessage,
    DaemonNotification,
    DaemonRequest,
    DaemonResponse,
    DaemonResponseData,
};

#[cfg(not(target_vendor = "apple"))]
const UNIX_SOCKET_PATH: &str = "/run/burrow.sock";

#[cfg(target_vendor = "apple")]
const UNIX_SOCKET_PATH: &str = "burrow.sock";

fn get_socket_path() -> String {
    if std::env::var("BURROW_SOCKET_PATH").is_ok() {
        return std::env::var("BURROW_SOCKET_PATH").unwrap();
    }
    UNIX_SOCKET_PATH.to_string()
}

pub struct Listener {
    cmd_tx: async_channel::Sender<DaemonCommand>,
    rsp_rx: async_channel::Receiver<DaemonResponse>,
    sub_chan: async_channel::Receiver<DaemonNotification>,
    inner: UnixListener,
}

impl Listener {
    #[throws]
    pub fn new(
        cmd_tx: async_channel::Sender<DaemonCommand>,
        rsp_rx: async_channel::Receiver<DaemonResponse>,
        sub_chan: async_channel::Receiver<DaemonNotification>,
    ) -> Self {
        let socket_path = get_socket_path();
        let path = Path::new(OsStr::new(&socket_path));
        Self::new_with_path(cmd_tx, rsp_rx, sub_chan, path)?
    }

    #[throws]
    #[cfg(target_os = "linux")]
    pub fn new_with_path(
        cmd_tx: async_channel::Sender<DaemonCommand>,
        rsp_rx: async_channel::Receiver<DaemonResponse>,
        sub_chan: async_channel::Receiver<DaemonNotification>,
        path: &Path,
    ) -> Self {
        let inner = listener_from_path_or_fd(&path, raw_fd())?;
        Self {
            cmd_tx,
            rsp_rx,
            sub_chan,
            inner,
        }
    }

    #[throws]
    #[cfg(not(target_os = "linux"))]
    pub fn new_with_path(
        cmd_tx: async_channel::Sender<DaemonCommand>,
        rsp_rx: async_channel::Receiver<DaemonResponse>,
        sub_chan: async_channel::Receiver<DaemonNotification>,
        path: &Path,
    ) -> Self {
        let inner = listener_from_path(path)?;
        Self {
            cmd_tx,
            rsp_rx,
            inner,
            sub_chan,
        }
    }

    pub async fn run(&self) -> Result<()> {
        info!("Waiting for connections...");
        loop {
            let (stream, _) = self.inner.accept().await?;
            let cmd_tx = self.cmd_tx.clone();
            let rsp_rxc = self.rsp_rx.clone();
            let sub_chan = self.sub_chan.clone();
            tokio::task::spawn(async move {
                info!("Got connection: {:?}", stream);
                Self::stream(stream, cmd_tx, rsp_rxc, sub_chan).await;
            });
        }
    }

    async fn stream(
        stream: UnixStream,
        cmd_tx: async_channel::Sender<DaemonCommand>,
        rsp_rxc: async_channel::Receiver<DaemonResponse>,
        sub_chan: async_channel::Receiver<DaemonNotification>,
    ) {
        let mut stream = stream;
        let (mut read_stream, mut write_stream) = stream.split();
        let buf_reader = BufReader::new(&mut read_stream);
        let mut lines = buf_reader.lines();
        loop {
            tokio::select! {
                Ok(Some(line)) = lines.next_line() => {
                    info!("Line: {}", line);
                    let mut res: DaemonResponse = DaemonResponseData::None.into();
                    let req = match serde_json::from_str::<DaemonRequest>(&line) {
                        Ok(req) => Some(req),
                        Err(e) => {
                            res.result = Err(e.to_string());
                            error!("Failed to parse request: {}", e);
                            None
                        }
                    };

                    let res = serde_json::to_string(&DaemonMessage::from(res)).unwrap();

                    if let Some(req) = req {
                        cmd_tx.send(req.command).await.unwrap();
                        let res = rsp_rxc.recv().await.unwrap().with_id(req.id);
                        let mut payload = serde_json::to_string(&DaemonMessage::from(res)).unwrap();
                        payload.push('\n');
                        info!("Sending response: {}", payload);
                        write_stream.write_all(payload.as_bytes()).await.unwrap();
                    } else {
                        write_stream.write_all(res.as_bytes()).await.unwrap();
                    }
                }
                Ok(cmd) = sub_chan.recv() => {
                    info!("Got subscription command: {:?}", cmd);
                    let msg = DaemonMessage::from(cmd);
                    let mut payload = serde_json::to_string(&msg).unwrap();
                    payload.push('\n');
                    write_stream.write_all(payload.as_bytes()).await.unwrap();
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn raw_fd() -> Option<RawFd> {
    if !libsystemd::daemon::booted() {
        return None;
    }

    match libsystemd::activation::receive_descriptors(false) {
        Ok(descriptors) => descriptors.into_iter().map(|d| d.into_raw_fd()).next(),
        Err(e) => {
            tracing::error!("Failed to receive descriptors: {}", e);
            None
        }
    }
}

#[throws]
#[cfg(target_os = "linux")]
fn listener_from_path_or_fd(path: &Path, raw_fd: Option<RawFd>) -> UnixListener {
    match raw_fd.map(listener_from_fd) {
        Some(Ok(listener)) => listener,
        _ => listener_from_path(path)?,
    }
}

#[throws]
#[cfg(target_os = "linux")]
fn listener_from_fd(fd: RawFd) -> UnixListener {
    use std::os::fd::FromRawFd;

    let listener = unsafe { std::os::unix::net::UnixListener::from_raw_fd(fd) };
    listener.set_nonblocking(true)?;
    UnixListener::from_std(listener)?
}

#[throws]
fn listener_from_path(path: &Path) -> UnixListener {
    let error = match UnixListener::bind(path) {
        Ok(listener) => return listener,
        Err(e) => e,
    };

    match error.kind() {
        io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                info!("Creating parent directory {:?}", parent);
                std::fs::create_dir_all(parent)?;
            }
        }
        io::ErrorKind::AddrInUse => {
            info!("Removing existing file");
            match std::fs::remove_file(path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                stuff => stuff,
            }?;
        }
        _ => error!("Failed to bind to {:?}: {}", path, error),
    }

    UnixListener::bind(path)?
}

#[derive(Debug)]
pub struct DaemonClient {
    stream: UnixStream,
}

impl DaemonClient {
    pub async fn new() -> Result<Self> {
        let socket_path = get_socket_path();
        let path = Path::new(OsStr::new(&socket_path));
        Self::new_with_path(path).await
    }

    pub async fn new_with_path(path: &Path) -> Result<Self> {
        let stream = UnixStream::connect(path).await?;
        Ok(Self { stream })
    }

    pub async fn send_command(&mut self, command: DaemonCommand) -> Result<DaemonResponse> {
        let mut command = serde_json::to_string(&DaemonRequest { id: 0, command })?;
        command.push('\n');

        self.stream.write_all(command.as_bytes()).await?;
        let buf_reader = BufReader::new(&mut self.stream);
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
