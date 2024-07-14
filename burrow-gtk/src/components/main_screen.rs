use super::*;

pub struct MainScreen {
    switch: AsyncController<main::Switch>,
}

pub struct MainScreenInit {
    pub daemon_client: Arc<Mutex<Option<Channel>>>,
}

#[derive(Debug)]
pub enum MainScreenMsg {
    None,
    DaemonDisconnect,
    DaemonReconnect,
}

#[relm4::component(pub, async)]
impl AsyncComponent for MainScreen {
    type Init = MainScreenInit;
    type Input = MainScreenMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_valign: Align::Fill,
            set_valign: Align::Center,

            // gtk::Box {
            //     set_orientation: gtk::Orientation::Vertical,
            //     set_spacing: 5,
            //     set_margin_all: 5,
            //     set_valign: Align::Start,

            //     #[name(setup_banner)]
            //     adw::Banner {
            //         set_title: "Burrow is not running!",
            //     },
            // },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 5,
                set_valign: Align::Center,
                set_vexpand: true,
            },

            #[name(content)]
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 5,
                set_valign: Align::Center,
                set_vexpand: true,

                gtk::Label {
                    set_label: "Main Screen",
                },
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let switch = main::Switch::builder()
            .launch(main::SwitchInit {
                daemon_client: Arc::clone(&init.daemon_client),
            })
            .forward(sender.input_sender(), |_| MainScreenMsg::None);

        let networks = main::Networks::builder()
            .launch(main::NetworksInit {
                daemon_client: Arc::clone(&init.daemon_client),
            })
            .forward(sender.input_sender(), |_| MainScreenMsg::None);

        let widgets = view_output!();

        widgets.content.append(networks.widget());
        widgets.content.append(switch.widget());

        let model = MainScreen { switch };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            MainScreenMsg::DaemonDisconnect => {
                self.switch.emit(main::SwitchMsg::DaemonDisconnect);
            }
            MainScreenMsg::DaemonReconnect => {
                self.switch.emit(main::SwitchMsg::DaemonReconnect);
            }
            _ => {}
        }
    }
}
