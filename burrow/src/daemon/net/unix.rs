use super::*;
use std::{
    os::fd::{FromRawFd, RawFd},
    os::unix::net::UnixListener as StdUnixListener,
    path::Path,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
};

const UNIX_SOCKET_PATH: &str = "/run/burrow.sock";

pub async fn listen(cmd_tx: mpsc::Sender<DaemonCommand>) -> Result<()> {
    listen_with_optional_fd(cmd_tx, None).await
}

pub(crate) async fn listen_with_optional_fd(
    cmd_tx: mpsc::Sender<DaemonCommand>,
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
        std::fs::remove_file(path)?;
        UnixListener::bind(path)?
    };
    loop {
        let (stream, _) = listener.accept().await?;
        let cmd_tx = cmd_tx.clone();

        //  I'm pretty sure we won't need to manually join / shut this down,
        //  `lines` will return Err during dropping, and this task should exit gracefully.
        tokio::task::spawn(async {
            let cmd_tx = cmd_tx;
            let mut stream = stream;
            let (mut read_stream, mut write_stream) = stream.split();
            let buf_reader = BufReader::new(&mut read_stream);
            let mut lines = buf_reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut res = DaemonResponse { result: Ok(()) };
                let command = match serde_json::from_str::<DaemonRequest>(&line) {
                    Ok(req) => Some(req.command),
                    Err(e) => {
                        res.result = Err(format!("{}", e));
                        None
                    }
                };
                let mut res = serde_json::to_string(&res).unwrap();
                res.push('\n');

                write_stream.write_all(res.as_bytes()).await.unwrap();

                //  I want this to come at the very end so that we always send a reponse back.
                if let Some(command) = command {
                    cmd_tx.send(command).await.unwrap();
                }
            }
        });
    }
}

pub struct DaemonClient {
    connection: UnixStream,
}

impl DaemonClient {
    pub async fn new() -> Result<Self> {
        Self::new_with_path(UNIX_SOCKET_PATH).await
    }

    pub async fn new_with_path(path: &str) -> Result<Self> {
        let path = Path::new(path);
        let connection = UnixStream::connect(path).await?;

        Ok(Self { connection })
    }

    pub async fn send_command(&mut self, command: DaemonCommand) -> Result<()> {
        let mut command = serde_json::to_string(&DaemonRequest { id: 0, command })?;
        command.push('\n');

        self.connection.write_all(command.as_bytes()).await?;
        let buf_reader = BufReader::new(&mut self.connection);
        let mut lines = buf_reader.lines();
        //  This unwrap *should* never cause issues.
        let response = lines.next_line().await?.unwrap();
        let res: DaemonResponse = serde_json::from_str(&response)?;
        res.result.unwrap();
        Ok(())
    }
}
