use super::*;

pub struct DaemonInstance {
    rx: mpsc::Receiver<DaemonCommand>,
    tun_interface: Option<TunInterface>,
}

impl DaemonInstance {
    pub fn new(rx: mpsc::Receiver<DaemonCommand>) -> Self {
        Self {
            rx,
            tun_interface: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        while let Some(command) = self.rx.recv().await {
            match command {
                DaemonCommand::Start(options) => {
                    if self.tun_interface.is_none() {
                        self.tun_interface = Some(options.tun.open()?);
                        eprintln!("Daemon starting tun interface.");
                    } else {
                        eprintln!("Got start, but tun interface already up.");
                    }
                }
                DaemonCommand::Stop => {
                    if self.tun_interface.is_some() {
                        self.tun_interface = None;
                        eprintln!("Daemon stopping tun interface.");
                    } else {
                        eprintln!("Got stop, but tun interface is not up.")
                    }
                }
            }
        }

        Ok(())
    }
}
