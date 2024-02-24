use super::*;
use std::process::Command;

#[derive(Debug)]
pub struct DaemonGroup {
    daemon_client: Arc<Mutex<Option<DaemonClient>>>,
    already_running: bool,
}

pub struct DaemonGroupInit {
    pub daemon_client: Arc<Mutex<Option<DaemonClient>>>,
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
            set_sensitive: diag::is_appimage() && !model.already_running,
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
        //  Should be impossible to panic here
        let model = DaemonGroup {
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
                let burrow_original_bin = std::env::vars()
                    .find(|(k, _)| k == "APPDIR")
                    .map(|(_, v)| v + "/usr/bin/burrow")
                    .unwrap_or("/usr/bin/burrow".to_owned());

                let mut burrow_bin =
                    String::from_utf8(Command::new("mktemp").output().unwrap().stdout).unwrap();
                burrow_bin.pop();

                let privileged_spawn_script = format!(
                    r#"TEMP=$(mktemp -p /root)
cp {} $TEMP
chmod +x $TEMP
setcap CAP_NET_BIND_SERVICE,CAP_NET_ADMIN+eip $TEMP
mv $TEMP /tmp/burrow-detached-daemon"#,
                    burrow_original_bin
                )
                .replace('\n', "&&");

                //  TODO: Handle error condition

                Command::new("pkexec")
                    .arg("sh")
                    .arg("-c")
                    .arg(privileged_spawn_script)
                    .arg(&burrow_bin)
                    .output()
                    .unwrap();

                Command::new("/tmp/burrow-detached-daemon")
                    .env("RUST_LOG", "debug")
                    .arg("daemon")
                    .spawn()
                    .unwrap();
            }
            DaemonGroupMsg::DaemonStateChange => {
                self.already_running = self.daemon_client.lock().await.is_some();
            }
        }
    }
}
