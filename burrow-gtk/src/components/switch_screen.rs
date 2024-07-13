use super::*;

pub struct SwitchScreen {
    daemon_client: Arc<Mutex<Option<Channel>>>,
    switch: gtk::Switch,
    switch_screen: gtk::Box,
    disconnected_banner: adw::Banner,
}

pub struct SwitchScreenInit {
    pub daemon_client: Arc<Mutex<Option<Channel>>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SwitchScreenMsg {
    DaemonReconnect,
    DaemonDisconnect,
    Start,
    Stop,
}

#[relm4::component(pub, async)]
impl AsyncComponent for SwitchScreen {
    type Init = SwitchScreenInit;
    type Input = SwitchScreenMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_valign: Align::Fill,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,
                set_valign: Align::Start,

                #[name(setup_banner)]
                adw::Banner {
                    set_title: "Burrow is not running!",
                },
            },

            #[name(switch_screen)]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 5,
                set_valign: Align::Center,
                set_vexpand: true,

                gtk::Label {
                    set_label: "Burrow Switch",
                },

                #[name(switch)]
                gtk::Switch {
                    set_halign: Align::Center,
                    set_hexpand: false,
                    set_vexpand: false,
                    connect_active_notify => move |switch|
                        sender.input(if switch.is_active() { SwitchScreenMsg::Start } else { SwitchScreenMsg::Stop })
                },
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let mut initial_switch_status = false;
        let mut initial_daemon_server_down = false;

        if let Some(daemon_client) = init.daemon_client.lock().await.as_mut() {
            let mut client = tunnel_client::TunnelClient::new(daemon_client);
            if let Ok(res) = client.tunnel_status(burrow_rpc::Empty {}).await.as_mut() {
                // TODO: RPC REFACTOR (Handle Error)
                let res = res.get_mut().message().await.unwrap().unwrap();
                initial_switch_status = match res.state() {
                    burrow_rpc::State::Running => true,
                    burrow_rpc::State::Stopped => false,
                };
            } else {
                initial_daemon_server_down = true;
            }
        } else {
            initial_daemon_server_down = true;
        }

        let widgets = view_output!();

        widgets.switch.set_active(initial_switch_status);

        if initial_daemon_server_down {
            *init.daemon_client.lock().await = None;
            widgets.switch.set_active(false);
            widgets.switch_screen.set_sensitive(false);
            widgets.setup_banner.set_revealed(true);
        }

        let model = SwitchScreen {
            daemon_client: init.daemon_client,
            switch: widgets.switch.clone(),
            switch_screen: widgets.switch_screen.clone(),
            disconnected_banner: widgets.setup_banner.clone(),
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let mut disconnected_daemon_client = false;

        if let Some(daemon_client) = self.daemon_client.lock().await.as_mut() {
            let mut client = tunnel_client::TunnelClient::new(daemon_client);
            match msg {
                Self::Input::Start => {
                    if let Err(_e) = client.tunnel_start(burrow_rpc::Empty {}).await {
                        disconnected_daemon_client = true;
                    }
                }
                Self::Input::Stop => {
                    if let Err(_e) = client.tunnel_stop(burrow_rpc::Empty {}).await {
                        disconnected_daemon_client = true;
                    }
                }
                _ => {}
            }
        } else {
            disconnected_daemon_client = true;
        }

        if msg == Self::Input::DaemonReconnect {
            self.disconnected_banner.set_revealed(false);
            self.switch_screen.set_sensitive(true);
        }

        if disconnected_daemon_client || msg == Self::Input::DaemonDisconnect {
            *self.daemon_client.lock().await = None;
            self.switch.set_active(false);
            self.switch_screen.set_sensitive(false);
            self.disconnected_banner.set_revealed(true);
        }
    }
}
