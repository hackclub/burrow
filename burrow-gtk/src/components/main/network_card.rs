use super::*;

pub struct NetworkCard {
    id: i32,
    index: usize,
    index_max: usize,
    daemon_client: Arc<Mutex<Option<Channel>>>,
}

pub struct NetworkCardInit {
    pub id: i32,
    pub index: usize,
    pub index_max: usize,
    pub name: String,
    pub enabled: bool,
    pub daemon_client: Arc<Mutex<Option<Channel>>>,
}

#[derive(Debug)]
pub enum NetworkCardMsg {
    NetworkDelete,
    MoveUp,
    MoveDown,
}

#[relm4::component(pub, async)]
impl AsyncComponent for NetworkCard {
    type Init = NetworkCardInit;
    type Input = NetworkCardMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::ListBoxRow {
            set_hexpand: true,
            set_halign: Align::Center,
            gtk::Box {
                gtk::Label {
                    set_label: &init.name
                },
                gtk::Switch {
                    set_halign: Align::Center,
                    set_hexpand: false,
                    set_vexpand: false,
                    set_state: init.enabled,
                },
                gtk::Button {
                    set_icon_name: "list-remove",
                    set_margin_all: 12,

                    connect_clicked => NetworkCardMsg::NetworkDelete,
                },
                gtk::Button {
                    set_icon_name: "pan-up-symbolic",
                    set_margin_all: 12,

                    connect_clicked => NetworkCardMsg::MoveUp,
                },
                gtk::Button {
                    set_icon_name: "pan-down-symbolic",
                    set_margin_all: 12,

                    connect_clicked => NetworkCardMsg::MoveDown,
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

        let model = NetworkCard {
            id: init.id,
            index: init.index,
            index_max: init.index_max,
            daemon_client: init.daemon_client,
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
            NetworkCardMsg::NetworkDelete => {
                if let Some(daemon_client) = self.daemon_client.lock().await.as_mut() {
                    let mut client = networks_client::NetworksClient::new(daemon_client);
                    client
                        .network_delete(burrow_rpc::NetworkDeleteRequest { id: self.id })
                        .await
                        .unwrap();
                }
            }
            NetworkCardMsg::MoveUp => {
                if self.index.checked_sub(1).is_some() {
                    if let Some(daemon_client) = self.daemon_client.lock().await.as_mut() {
                        let mut client = networks_client::NetworksClient::new(daemon_client);
                        client
                            .network_reorder(burrow_rpc::NetworkReorderRequest {
                                id: self.id,
                                index: self.index as i32 - 1,
                            })
                            .await
                            .unwrap();
                    }
                }
            }
            NetworkCardMsg::MoveDown => {
                if self.index + 1 < self.index_max {
                    if let Some(daemon_client) = self.daemon_client.lock().await.as_mut() {
                        let mut client = networks_client::NetworksClient::new(daemon_client);
                        client
                            .network_reorder(burrow_rpc::NetworkReorderRequest {
                                id: self.id,
                                index: self.index as i32 + 1,
                            })
                            .await
                            .unwrap();
                    }
                }
            }
        }
    }
}
