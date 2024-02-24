use super::*;
use diag::{StatusTernary, SystemSetup};

#[derive(Debug)]
pub struct DiagGroup {
    daemon_client: Arc<Mutex<Option<DaemonClient>>>,

    init_system: SystemSetup,
    service_installed: StatusTernary,
    socket_installed: StatusTernary,
    socket_enabled: StatusTernary,
    daemon_running: bool,
}

pub struct DiagGroupInit {
    pub daemon_client: Arc<Mutex<Option<DaemonClient>>>,
}

impl DiagGroup {
    async fn new(daemon_client: Arc<Mutex<Option<DaemonClient>>>) -> Result<Self> {
        let setup = SystemSetup::new();
        let daemon_running = daemon_client.lock().await.is_some();

        Ok(Self {
            service_installed: setup.is_service_installed()?,
            socket_installed: setup.is_socket_installed()?,
            socket_enabled: setup.is_socket_enabled()?,
            daemon_running,
            init_system: setup,
            daemon_client,
        })
    }
}

#[derive(Debug)]
pub enum DiagGroupMsg {
    Refresh,
}

#[relm4::component(pub, async)]
impl AsyncComponent for DiagGroup {
    type Init = DiagGroupInit;
    type Input = DiagGroupMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        #[name(group)]
        adw::PreferencesGroup {
            set_title: "Diagnose",
            set_description: Some("Diagnose Burrow"),

            adw::ActionRow {
                #[watch]
                set_title: &format!("System Type: {}", model.init_system)
            },
            adw::ActionRow {
                #[watch]
                set_title: &format!(
                    "Service installed: {}",
                    status_ternary_to_str(model.service_installed)
                )
            },
            adw::ActionRow {
                #[watch]
                set_title: &format!(
                    "Socket installed: {}",
                    status_ternary_to_str(model.socket_installed)
                )
            },
            adw::ActionRow {
                #[watch]
                set_title: &format!(
                    "Socket enabled: {}",
                    status_ternary_to_str(model.socket_enabled)
                )
            },
            adw::ActionRow {
                #[watch]
                set_title: &format!(
                    "Daemon running: {}",
                    if model.daemon_running { "Yes" } else { "No" }
                )
            },
            gtk::Button {
                set_label: "Refresh",
                connect_clicked => DiagGroupMsg::Refresh
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        //  Should be impossible to panic here
        let model = DiagGroup::new(init.daemon_client).await.unwrap();

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
            DiagGroupMsg::Refresh => {
                //  Should be impossible to panic here
                *self = Self::new(Arc::clone(&self.daemon_client)).await.unwrap();
            }
        }
    }
}

fn status_ternary_to_str(status: StatusTernary) -> &'static str {
    match status {
        StatusTernary::True => "Yes",
        StatusTernary::False => "No",
        StatusTernary::NA => "N/A",
    }
}
