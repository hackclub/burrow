use super::*;
use std::time::Duration;

const RECONNECT_POLL_TIME: Duration = Duration::from_secs(3);

pub struct Networks {
    daemon_client: Arc<Mutex<Option<Channel>>>,
    network_cards: Vec<AsyncController<NetworkCard>>,
    networks_list_box: gtk::ListBox,

    _network_state_worker: WorkerController<AsyncNetworkStateHandler>,
}

pub struct NetworksInit {
    pub daemon_client: Arc<Mutex<Option<Channel>>>,
}

#[derive(Debug)]
pub enum NetworksMsg {
    None,
    NetworkList(Vec<burrow_rpc::Network>),
    NetworkAdd,
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
            set_spacing: 20,
            set_margin_all: 5,
            set_valign: Align::Fill,
            set_vexpand: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 10,
                set_margin_all: 5,
                set_valign: Align::Start,
                set_halign: Align::Center,

                gtk::Label {
                    set_label: "Add Network",
                },

                gtk::Button {
                    set_icon_name: "list-add",
                    set_margin_all: 12,

                    connect_clicked => NetworksMsg::NetworkAdd,
                },
            },


            gtk::ScrolledWindow {
                set_valign: Align::Fill,
                set_vexpand: true,
                set_margin_bottom: 50,
                set_margin_start: 50,
                set_margin_end: 50,

                #[name = "networks"]
                gtk::ListBox {
                    set_vexpand: true,
                },
            }
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
                    id: 0,
                    index: 0,
                    index_max: 3,
                    daemon_client: Arc::clone(&init.daemon_client),
                    name: "Hello".to_owned(),
                    enabled: true,
                })
                .forward(sender.input_sender(), |_| NetworksMsg::None),
            NetworkCard::builder()
                .launch(NetworkCardInit {
                    id: 1,
                    index: 1,
                    index_max: 3,
                    daemon_client: Arc::clone(&init.daemon_client),
                    name: "World".to_owned(),
                    enabled: false,
                })
                .forward(sender.input_sender(), |_| NetworksMsg::None),
            NetworkCard::builder()
                .launch(NetworkCardInit {
                    id: 2,
                    index: 2,
                    index_max: 3,
                    daemon_client: Arc::clone(&init.daemon_client),
                    name: "Yay".to_owned(),
                    enabled: false,
                })
                .forward(sender.input_sender(), |_| NetworksMsg::None),
            NetworkCard::builder()
                .launch(NetworkCardInit {
                    id: 2,
                    index: 2,
                    index_max: 3,
                    daemon_client: Arc::clone(&init.daemon_client),
                    name: "Yay".to_owned(),
                    enabled: false,
                })
                .forward(sender.input_sender(), |_| NetworksMsg::None),
            NetworkCard::builder()
                .launch(NetworkCardInit {
                    id: 2,
                    index: 2,
                    index_max: 3,
                    daemon_client: Arc::clone(&init.daemon_client),
                    name: "Yay".to_owned(),
                    enabled: false,
                })
                .forward(sender.input_sender(), |_| NetworksMsg::None),
        ];
        for network_card in network_cards.iter() {
            widgets.networks.append(network_card.widget());
        }
        // let network_cards = vec![];

        let model = Networks {
            daemon_client: init.daemon_client,
            network_cards,
            networks_list_box: widgets.networks.clone(),

            _network_state_worker: AsyncNetworkStateHandler::builder()
                .detach_worker(())
                .forward(sender.input_sender(), |msg| msg),
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            NetworksMsg::NetworkList(networks) => {
                for network_card in self.network_cards.iter() {
                    self.networks_list_box
                        .remove(&network_card.widget().clone());
                }
                self.network_cards.clear();

                let index_max = networks.len();
                for (index, network) in networks.iter().enumerate() {
                    let network_card = NetworkCard::builder()
                        .launch(NetworkCardInit {
                            id: network.id,
                            index,
                            index_max,
                            daemon_client: Arc::clone(&self.daemon_client),
                            name: format!("ID: {}, TYPE: {}", network.id, network.r#type),
                            enabled: false,
                        })
                        .forward(sender.input_sender(), |_| NetworksMsg::None);
                    self.networks_list_box.append(network_card.widget());
                    self.network_cards.push(network_card);
                }
            }
            NetworksMsg::NetworkAdd => {
                if let Some(daemon_client) = self.daemon_client.lock().await.as_mut() {
                    let mut client = networks_client::NetworksClient::new(daemon_client);
                    let _ = client.network_add(burrow_rpc::Empty {}).await;
                }
            }
            _ => {}
        }
    }
}

struct AsyncNetworkStateHandler;

impl Worker for AsyncNetworkStateHandler {
    type Init = ();
    type Input = ();
    type Output = NetworksMsg;

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
