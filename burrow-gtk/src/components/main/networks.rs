use super::*;
use std::time::Duration;

const RECONNECT_POLL_TIME: Duration = Duration::from_secs(3);

pub struct Networks {
    daemon_client: Arc<Mutex<Option<Channel>>>,
    network_cards: Vec<AsyncController<NetworkCard>>,
    networks_list_box: gtk::ListBox,
}

pub struct NetworksInit {
    pub daemon_client: Arc<Mutex<Option<Channel>>>,
}

#[derive(Debug)]
pub enum NetworksMsg {
    None,
    NetworkList(Vec<burrow_rpc::Network>),
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

        let network_cards = vec![
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

        let model = Networks {
            daemon_client: init.daemon_client,
            network_cards,
            networks_list_box: widgets.networks.clone(),
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if let NetworksMsg::NetworkList(networks) = msg {
            for network_card in self.network_cards.iter() {
                self.networks_list_box
                    .remove(&network_card.widget().clone());
            }
            self.network_cards.clear();

            for network in networks {
                let network_card = NetworkCard::builder()
                    .launch(NetworkCardInit {
                        name: format!("ID: {}, TYPE: {}", network.id, network.r#type),
                        enabled: false,
                    })
                    .forward(sender.input_sender(), |_| NetworksMsg::None);
                self.networks_list_box.append(network_card.widget());
                self.network_cards.push(network_card);
            }
        }
    }
}

struct AsyncNetworkStateHandler;

impl Worker for AsyncNetworkStateHandler {
    type Init = ();
    type Input = ();
    type Output = NetworksMsg;

    fn init(_: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self
    }

    fn update(&mut self, _: (), sender: ComponentSender<Self>) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let task = rt.spawn(async move {
            loop {
                let conn = daemon::daemon_connect().await;
                if let Ok(conn) = conn {
                    let mut client = networks_client::NetworksClient::new(conn);
                    if let Ok(mut res) = client.network_list(burrow_rpc::Empty {}).await {
                        let stream = res.get_mut();
                        while let Ok(Some(msg)) = stream.message().await {
                            sender
                                .output(NetworksMsg::NetworkList(msg.network))
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
