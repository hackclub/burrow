use super::*;
use std::{fmt::Display, fs, process::Command};

const SYSTEMD_SOCKET_LOC: &str = "/etc/systemd/system/burrow.socket";
const SYSTEMD_SERVICE_LOC: &str = "/etc/systemd/system/burrow.service";

//  I don't like this type very much.
#[derive(Debug, Clone, Copy)]
pub enum StatusTernary {
    True,
    False,
    NA,
}

//  Realistically, we may not explicitly "support" non-systemd platforms which would simply this
//  code greatly.
//  Along with replacing [`StatusTernary`] with good old [`bool`].
#[derive(Debug, Clone, Copy)]
pub enum SystemSetup {
    Systemd,
    Other,
}

impl SystemSetup {
    pub fn new() -> Self {
        if Command::new("systemctl").arg("--version").output().is_ok() {
            SystemSetup::Systemd
        } else {
            SystemSetup::Other
        }
    }

    pub fn is_service_installed(&self) -> Result<StatusTernary> {
        match self {
            SystemSetup::Systemd => Ok(fs::metadata(SYSTEMD_SERVICE_LOC).is_ok().into()),
            SystemSetup::Other => Ok(StatusTernary::NA),
        }
    }

    pub fn is_socket_installed(&self) -> Result<StatusTernary> {
        match self {
            SystemSetup::Systemd => Ok(fs::metadata(SYSTEMD_SOCKET_LOC).is_ok().into()),
            SystemSetup::Other => Ok(StatusTernary::NA),
        }
    }

    pub fn is_socket_enabled(&self) -> Result<StatusTernary> {
        match self {
            SystemSetup::Systemd => {
                let output = Command::new("systemctl")
                    .arg("is-enabled")
                    .arg("burrow.socket")
                    .output()?
                    .stdout;
                let output = String::from_utf8(output)?;
                Ok((output == "enabled\n").into())
            }
            SystemSetup::Other => Ok(StatusTernary::NA),
        }
    }
}

impl From<bool> for StatusTernary {
    fn from(value: bool) -> Self {
        if value {
            StatusTernary::True
        } else {
            StatusTernary::False
        }
    }
}

impl Display for SystemSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SystemSetup::Systemd => "Systemd",
            SystemSetup::Other => "Other",
        })
    }
}
