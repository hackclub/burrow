use super::*;

pub struct Networks {
    daemon_client: Arc<Mutex<Option<Channel>>>,
    network_widgets: Vec<AsyncController<NetworkCard>>,
}

pub struct NetworksInit {
    pub daemon_client: Arc<Mutex<Option<Channel>>>,
}

#[derive(Debug)]
pub enum NetworksMsg {
    None,
}

#[relm4::component(pub, async)]
impl AsyncComponent for Networks {
    type Init = NetworksInit;
    type Input = NetworksMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,
            set_margin_all: 5,
            set_valign: Align::Start,

            #[name = "networks"]
            gtk::ListBox {}
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let widgets = view_output!();

        let network_widgets = vec![
            NetworkCard::builder()
                .launch(NetworkCardInit {
                    name: "Hello".to_owned(),
                    enabled: true,
                })
                .forward(sender.input_sender(), |_| NetworksMsg::None),
            NetworkCard::builder()
                .launch(NetworkCardInit {
                    name: "World".to_owned(),
                    enabled: false,
                })
                .forward(sender.input_sender(), |_| NetworksMsg::None),
            NetworkCard::builder()
                .launch(NetworkCardInit {
                    name: "Yay".to_owned(),
                    enabled: false,
                })
                .forward(sender.input_sender(), |_| NetworksMsg::None),
        ];

        widgets.networks.append(network_widgets[0].widget());
        widgets.networks.append(network_widgets[1].widget());
        widgets.networks.append(network_widgets[2].widget());

        let model = Networks {
            daemon_client: init.daemon_client,
            network_widgets,
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
    }
}
