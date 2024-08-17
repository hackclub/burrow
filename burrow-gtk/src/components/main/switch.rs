use super::*;
use std::time::Duration;

const RECONNECT_POLL_TIME: Duration = Duration::from_secs(3);

pub struct Switch {
    daemon_client: Arc<Mutex<Option<Channel>>>,
    switch: gtk::Switch,

    _tunnel_state_worker: WorkerController<AsyncTunnelStateHandler>,
}

pub struct SwitchInit {
    pub daemon_client: Arc<Mutex<Option<Channel>>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SwitchMsg {
    None,
    Start,
    Stop,
    SwitchSetStart,
    SwitchSetStop,
}

#[relm4::component(pub, async)]
impl AsyncComponent for Switch {
    type Init = SwitchInit;
    type Input = SwitchMsg;
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
                        switch_sender.input(if switch.is_active() { SwitchMsg::Start } else { SwitchMsg::Stop })
                },
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let switch_sender = sender.clone();
        let widgets = view_output!();

        let model = Switch {
            daemon_client: init.daemon_client,
            switch: widgets.switch.clone(),
            _tunnel_state_worker: AsyncTunnelStateHandler::builder()
                .detach_worker(())
                .forward(sender.input_sender(), |_| SwitchMsg::None),
        };

        widgets.switch.set_active(false);

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if let Some(daemon_client) = self.daemon_client.lock().await.as_mut() {
            let mut client = tunnel_client::TunnelClient::new(daemon_client);
            match msg {
                Self::Input::Start => {
                    // TODO: Figure out best way for error handling.
                    let _ = client.tunnel_start(burrow_rpc::Empty {}).await;
                }
                Self::Input::Stop => {
                    // TODO: Figure out best way for error handling.
                    let _ = client.tunnel_stop(burrow_rpc::Empty {}).await;
                }
                Self::Input::SwitchSetStart => {
                    self.switch.set_active(true);
                }
                Self::Input::SwitchSetStop => {
                    self.switch.set_active(false);
                }
                _ => {}
            }
        }
    }
}

struct AsyncTunnelStateHandler;

impl Worker for AsyncTunnelStateHandler {
    type Init = ();
    type Input = ();
    type Output = SwitchMsg;

    fn init(_: Self::Init, sender: ComponentSender<Self>) -> Self {
        sender.input(());
        Self
    }

    fn update(&mut self, _: (), sender: ComponentSender<Self>) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let task = rt.spawn(async move {
            loop {
                let conn = daemon::daemon_connect().await;
                if let Ok(conn) = conn {
                    let mut client = tunnel_client::TunnelClient::new(conn);
                    if let Ok(mut res) = client.tunnel_status(burrow_rpc::Empty {}).await {
                        let stream = res.get_mut();
                        while let Ok(Some(msg)) = stream.message().await {
                            sender
                                .output(match msg.state() {
                                    burrow_rpc::State::Running => SwitchMsg::SwitchSetStart,
                                    burrow_rpc::State::Stopped => SwitchMsg::SwitchSetStop,
                                })
                                .unwrap();
                        }
                    }
                }
                tokio::time::sleep(RECONNECT_POLL_TIME).await;
            }
        });
        rt.block_on(task).unwrap();
    }
}
