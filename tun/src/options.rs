use std::io::Error;

use fehler::throws;

#[cfg(any(target_os = "linux", target_vendor = "apple"))]
#[cfg(feature = "tokio")]
use super::tokio::TunInterface;

#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct TunOptions {
    /// (Windows + Linux) Name the tun interface.
    pub name: Option<String>,
    /// (Linux) Don't include packet information.
    pub no_pi: bool,
    /// (Linux) Avoid opening an existing persistant device.
    pub tun_excl: bool,
    /// (Apple) Retrieve the tun interface
    pub tun_retrieve: bool,
    /// (Linux) The IP address of the tun interface.
    pub address: Vec<String>,
}

impl TunOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    pub fn no_pi(mut self, enable: bool) -> Self {
        self.no_pi = enable;
        self
    }

    pub fn tun_excl(mut self, enable: bool) -> Self {
        self.tun_excl = enable;
        self
    }

    pub fn address(mut self, address: Vec<impl ToString>) -> Self {
        self.address = address.iter().map(|x| x.to_string()).collect();
        self
    }

    #[cfg(any(target_os = "linux", target_vendor = "apple"))]
    #[cfg(feature = "tokio")]
    #[throws]
    pub fn open(self) -> TunInterface {
        let ti = super::TunInterface::new_with_options(self)?;
        TunInterface::new(ti)?
    }
}
