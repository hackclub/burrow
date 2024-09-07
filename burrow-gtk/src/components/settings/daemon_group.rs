use super::*;
use std::process::Command;

#[derive(Debug)]
pub struct DaemonGroup {
    system_setup: SystemSetup,
    daemon_client: Arc<Mutex<Option<Channel>>>,
    already_running: bool,
}

pub struct DaemonGroupInit {
    pub daemon_client: Arc<Mutex<Option<Channel>>>,
    pub system_setup: SystemSetup,
}

#[derive(Debug)]
pub enum DaemonGroupMsg {
    LaunchLocal,
    DaemonStateChange,
}

#[relm4::component(pub, async)]
impl AsyncComponent for DaemonGroup {
    type Init = DaemonGroupInit;
    type Input = DaemonGroupMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[name(group)]
        adw::PreferencesGroup {
            #[watch]
            set_sensitive:
                (model.system_setup == SystemSetup::AppImage || model.system_setup == SystemSetup::Other) &&
                !model.already_running,
            set_title: "Local Daemon",
            set_description: Some("Run Local Daemon"),

            gtk::Button {
                set_label: "Launch",
                connect_clicked => DaemonGroupMsg::LaunchLocal
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = DaemonGroup {
            system_setup: init.system_setup,
            daemon_client: init.daemon_client.clone(),
            already_running: init.daemon_client.lock().await.is_some(),
        };

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            DaemonGroupMsg::LaunchLocal => {
                if let Err(e) = launch_local() {
                    error!("Failed to launch local daemon at: {}", e);
                };
            }
            DaemonGroupMsg::DaemonStateChange => {
                self.already_running = self.daemon_client.lock().await.is_some();
            }
        }
    }
}

fn launch_local() -> Result<()> {
    const BURROW_LOCAL_DAEMON_PATH: &str = "/tmp/burrow-detached-daemon";

    let burrow_original_bin = std::env::vars()
        .find(|(k, _)| k == "APPDIR")
        .map(|(_, v)| v + "/usr/bin/burrow")
        .unwrap_or("/usr/bin/burrow".to_owned());

    Command::new("cp")
        .arg(&burrow_original_bin)
        .arg(BURROW_LOCAL_DAEMON_PATH)
        .output()
        .with_context(|| {
            format!(
                "Copying {} to {}",
                burrow_original_bin, BURROW_LOCAL_DAEMON_PATH
            )
        })?;

    let mut burrow_bin = String::from_utf8(Command::new("mktemp").output()?.stdout)?;
    burrow_bin.pop();

    let privileged_spawn_script = format!(
        r#"chmod +x {}
setcap CAP_NET_BIND_SERVICE,CAP_NET_ADMIN+eip {}"#,
        BURROW_LOCAL_DAEMON_PATH, BURROW_LOCAL_DAEMON_PATH
    )
    .replace('\n', "&&");

    //  Need to be more careful here.
    Command::new("pkexec")
        .arg("sh")
        .arg("-c")
        .arg(privileged_spawn_script)
        .arg(&burrow_bin)
        .output()
        .with_context(|| format!("Priviledged call to {}", burrow_bin))?;

    Command::new(BURROW_LOCAL_DAEMON_PATH)
        .env("RUST_LOG", "debug")
        .arg("daemon")
        .spawn()?;

    Ok(())
}
