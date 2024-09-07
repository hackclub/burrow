use super::*;
use std::{fmt::Display, fs, process::Command};

const SYSTEMD_SOCKET_LOC: &str = "/etc/systemd/system/burrow.socket";
const SYSTEMD_SERVICE_LOC: &str = "/etc/systemd/system/burrow.service";

#[derive(Debug, Clone, Copy)]
pub enum StatusTernary {
    True,
    False,
    NA,
}

//  Realistically, we may not explicitly "support" non-systemd platforms which would simply this
//  code greatly.
//  Along with replacing [`StatusTernary`] with good old [`bool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemSetup {
    Systemd,
    AppImage,
    Other,
}

impl SystemSetup {
    pub fn new() -> Self {
        if is_appimage() {
            SystemSetup::AppImage
        } else if Command::new("systemctl").arg("--version").output().is_ok() {
            SystemSetup::Systemd
        } else {
            SystemSetup::Other
        }
    }

    pub fn is_service_installed(&self) -> StatusTernary {
        match self {
            SystemSetup::Systemd => fs::metadata(SYSTEMD_SERVICE_LOC).is_ok().into(),
            SystemSetup::AppImage => StatusTernary::NA,
            SystemSetup::Other => StatusTernary::NA,
        }
    }

    pub fn is_socket_installed(&self) -> StatusTernary {
        match self {
            SystemSetup::Systemd => fs::metadata(SYSTEMD_SOCKET_LOC).is_ok().into(),
            SystemSetup::AppImage => StatusTernary::NA,
            SystemSetup::Other => StatusTernary::NA,
        }
    }

    pub fn is_socket_enabled(&self) -> StatusTernary {
        match self {
            SystemSetup::Systemd => {
                let Ok(output) = Command::new("systemctl")
                    .arg("is-enabled")
                    .arg("burrow.socket")
                    .output()
                    .map(|o| o.stdout)
                    .inspect_err(|e| {
                        error!("Failed to run `systemctl is-enabled burrow.socket` {}", e)
                    })
                else {
                    return StatusTernary::NA;
                };
                let output = String::from_utf8(output).unwrap();
                (output == "enabled\n").into()
            }
            SystemSetup::AppImage => StatusTernary::NA,
            SystemSetup::Other => StatusTernary::NA,
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
            SystemSetup::AppImage => "AppImage",
            SystemSetup::Other => "Other",
        })
    }
}

pub fn is_appimage() -> bool {
    std::env::vars().any(|(k, _)| k == "APPDIR")
}
