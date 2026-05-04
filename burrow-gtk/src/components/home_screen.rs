use super::*;
use crate::account_store::{self, AccountKind, AccountRecord};
use std::time::Duration;

pub struct HomeScreen {
    daemon_banner: adw::Banner,
    network_status: gtk::Label,
    network_cards: gtk::Box,
    account_status: gtk::Label,
    account_rows: gtk::Box,
    tunnel_status: gtk::Label,
    tunnel_button: gtk::Button,
    tunnel_state: Option<daemon_api::TunnelState>,
    tailnet_session_id: Option<String>,
    tailnet_running: bool,
}

#[derive(Debug)]
pub enum HomeScreenMsg {
    EnsureDaemon,
    Refresh,
    TunnelAction,
    OpenWireGuard,
    OpenTor,
    OpenTailnet,
    AddWireGuard {
        title: String,
        account: String,
        identity: String,
        config: String,
    },
    SaveTor {
        title: String,
        account: String,
        identity: String,
        note: String,
    },
    DiscoverTailnet(String),
    ProbeTailnet(String),
    StartTailnetLogin {
        authority: String,
        account: String,
        identity: String,
        hostname: Option<String>,
    },
    PollTailnetLogin,
    CancelTailnetLogin,
    AddTailnet {
        authority: String,
        account: String,
        identity: String,
        hostname: Option<String>,
        tailnet: Option<String>,
    },
}

#[relm4::component(pub, async)]
impl AsyncComponent for HomeScreen {
    type Init = ();
    type Input = HomeScreenMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,

            adw::Clamp {
                set_maximum_size: 900,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 24,
                    set_margin_all: 24,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 16,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 6,
                            set_hexpand: true,

                            gtk::Label {
                                add_css_class: "title-1",
                                set_xalign: 0.0,
                                set_label: "Burrow",
                            },

                            gtk::Label {
                                add_css_class: "heading",
                                add_css_class: "dim-label",
                                set_xalign: 0.0,
                                set_label: "Networks and accounts",
                            },
                        },

                        #[name(add_button)]
                        gtk::MenuButton {
                            add_css_class: "flat",
                            set_icon_name: "list-add-symbolic",
                            set_tooltip_text: Some("Add"),
                            set_valign: Align::Start,
                        },
                    },

                    #[name(daemon_banner)]
                    adw::Banner {
                        set_title: "Starting Burrow daemon",
                        set_revealed: false,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 4,

                            gtk::Label {
                                add_css_class: "title-2",
                                set_xalign: 0.0,
                                set_label: "Networks",
                            },

                            #[name(network_status)]
                            gtk::Label {
                                add_css_class: "dim-label",
                                set_xalign: 0.0,
                                set_wrap: true,
                                set_label: "Stored daemon networks and their active account selectors",
                            },
                        },

                        gtk::ScrolledWindow {
                            set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Never),
                            set_min_content_height: 190,

                            #[name(network_cards)]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 14,
                            },
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 4,

                            gtk::Label {
                                add_css_class: "title-2",
                                set_xalign: 0.0,
                                set_label: "Accounts",
                            },

                            gtk::Label {
                                add_css_class: "dim-label",
                                set_xalign: 0.0,
                                set_wrap: true,
                                set_label: "Per-network identities and sign-in state",
                            },
                        },

                        #[name(account_rows)]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                            set_margin_all: 0,
                            set_valign: Align::Center,
                        },

                        #[name(account_status)]
                        gtk::Label {
                            add_css_class: "dim-label",
                            set_xalign: 0.0,
                            set_wrap: true,
                            set_label: "",
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 4,

                            gtk::Label {
                                add_css_class: "title-2",
                                set_xalign: 0.0,
                                set_label: "Tunnel",
                            },

                            gtk::Label {
                                add_css_class: "dim-label",
                                set_xalign: 0.0,
                                set_label: "Current daemon tunnel state",
                            },
                        },

                        #[name(tunnel_status)]
                        gtk::Label {
                            set_xalign: 0.0,
                            set_label: "Checking daemon status",
                        },

                        #[name(tunnel_button)]
                        gtk::Button {
                            add_css_class: "suggested-action",
                            set_label: "Start",
                            set_halign: Align::Start,
                            connect_clicked => HomeScreenMsg::TunnelAction,
                        },
                    },
                }
            }
        }
    }

    async fn init(
        _: Self::Init,
        _root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let widgets = view_output!();
        configure_add_popover(&widgets.add_button, &sender);

        let refresh_sender = sender.input_sender().clone();
        relm4::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                refresh_sender.emit(HomeScreenMsg::Refresh);
            }
        });

        let model = HomeScreen {
            daemon_banner: widgets.daemon_banner.clone(),
            network_status: widgets.network_status.clone(),
            network_cards: widgets.network_cards.clone(),
            account_status: widgets.account_status.clone(),
            account_rows: widgets.account_rows.clone(),
            tunnel_status: widgets.tunnel_status.clone(),
            tunnel_button: widgets.tunnel_button.clone(),
            tunnel_state: None,
            tailnet_session_id: None,
            tailnet_running: false,
        };

        sender.input(HomeScreenMsg::EnsureDaemon);

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            HomeScreenMsg::EnsureDaemon => self.ensure_daemon().await,
            HomeScreenMsg::Refresh => self.refresh().await,
            HomeScreenMsg::TunnelAction => self.perform_tunnel_action().await,
            HomeScreenMsg::OpenWireGuard => open_wireguard_window(root, &sender),
            HomeScreenMsg::OpenTor => open_tor_window(root, &sender),
            HomeScreenMsg::OpenTailnet => open_tailnet_window(root, &sender),
            HomeScreenMsg::AddWireGuard {
                title,
                account,
                identity,
                config,
            } => self.add_wireguard(title, account, identity, config).await,
            HomeScreenMsg::SaveTor { title, account, identity, note } => {
                self.save_tor(title, account, identity, note)
            }
            HomeScreenMsg::DiscoverTailnet(email) => self.discover_tailnet(email).await,
            HomeScreenMsg::ProbeTailnet(authority) => self.probe_tailnet(authority).await,
            HomeScreenMsg::StartTailnetLogin {
                authority,
                account,
                identity,
                hostname,
            } => {
                self.start_tailnet_login(authority, account, identity, hostname, sender)
                    .await;
            }
            HomeScreenMsg::PollTailnetLogin => self.poll_tailnet_login(sender).await,
            HomeScreenMsg::CancelTailnetLogin => self.cancel_tailnet_login().await,
            HomeScreenMsg::AddTailnet {
                authority,
                account,
                identity,
                hostname,
                tailnet,
            } => {
                self.add_tailnet(authority, account, identity, hostname, tailnet)
                    .await;
            }
        }
    }
}

impl HomeScreen {
    async fn ensure_daemon(&mut self) {
        self.daemon_banner.set_title("Starting Burrow daemon");
        self.daemon_banner.set_revealed(true);
        match daemon_api::ensure_daemon().await {
            Ok(()) => {
                self.daemon_banner.set_revealed(false);
                self.refresh().await;
            }
            Err(error) => {
                self.daemon_banner
                    .set_title(&format!("Burrow daemon is not reachable: {error}"));
                self.daemon_banner.set_revealed(true);
                self.tunnel_state = None;
                self.tunnel_status.set_label("Daemon unavailable");
                self.tunnel_button.set_label("Enable");
                self.tunnel_button.set_sensitive(true);
                self.network_status
                    .set_label("Stored daemon networks are unavailable until the daemon starts.");
                self.render_networks(&[]);
            }
        }
    }

    async fn refresh(&mut self) {
        match daemon_api::tunnel_state().await {
            Ok(state) => {
                self.daemon_banner.set_revealed(false);
                self.tunnel_state = Some(state);
                match state {
                    daemon_api::TunnelState::Running => {
                        self.tunnel_status.set_label("Connected");
                        self.tunnel_button.set_label("Stop");
                    }
                    daemon_api::TunnelState::Stopped => {
                        self.tunnel_status.set_label("Disconnected");
                        self.tunnel_button.set_label("Start");
                    }
                }
                self.tunnel_button.set_sensitive(true);
            }
            Err(error) => {
                self.tunnel_state = None;
                self.daemon_banner
                    .set_title(&format!("Burrow daemon is not reachable: {error}"));
                self.daemon_banner.set_revealed(true);
                self.tunnel_status.set_label("Unknown");
                self.tunnel_button.set_label("Enable");
                self.tunnel_button.set_sensitive(true);
            }
        }

        match daemon_api::list_networks().await {
            Ok(networks) => {
                self.render_networks(&networks);
                self.network_status.set_label(if networks.is_empty() {
                    "Stored daemon networks and their active account selectors"
                } else {
                    "Stored daemon networks and their active account selectors"
                });
            }
            Err(error) => {
                self.render_networks(&[]);
                self.network_status
                    .set_label(&format!("Unable to read daemon networks: {error}"));
            }
        }

        match account_store::load() {
            Ok(accounts) => {
                self.account_status.set_label("");
                self.render_accounts(&accounts);
            }
            Err(error) => {
                self.render_accounts(&[]);
                self.account_status
                    .set_label(&format!("Unable to read account store: {error}"));
            }
        }
    }

    async fn perform_tunnel_action(&mut self) {
        match self.tunnel_state {
            Some(daemon_api::TunnelState::Running) => {
                self.tunnel_button.set_sensitive(false);
                self.tunnel_status.set_label("Disconnecting...");
                if let Err(error) = daemon_api::stop_tunnel().await {
                    self.tunnel_status
                        .set_label(&format!("Stop failed: {error}"));
                }
                self.refresh().await;
            }
            Some(daemon_api::TunnelState::Stopped) => {
                self.tunnel_button.set_sensitive(false);
                self.tunnel_status.set_label("Connecting...");
                if let Err(error) = daemon_api::start_tunnel().await {
                    self.tunnel_status
                        .set_label(&format!("Start failed: {error}"));
                }
                self.refresh().await;
            }
            None => self.ensure_daemon().await,
        }
    }

    async fn add_wireguard(
        &mut self,
        title: String,
        account: String,
        identity: String,
        config: String,
    ) {
        if config.trim().is_empty() {
            self.network_status
                .set_label("Paste a WireGuard configuration before adding a network.");
            return;
        }
        match daemon_api::add_wireguard(config).await {
            Ok(id) => {
                let title = daemon_api::normalized(&title, &format!("WireGuard {id}"));
                let record = account_store::new_record(
                    AccountKind::WireGuard,
                    title,
                    None,
                    daemon_api::normalized(&account, "default"),
                    daemon_api::normalized(&identity, &format!("network-{id}")),
                    None,
                    None,
                    Some(format!("Linked to daemon network #{id}.")),
                );
                match account_store::upsert(record) {
                    Ok(accounts) => self.render_accounts(&accounts),
                    Err(error) => self
                        .account_status
                        .set_label(&format!("WireGuard account save failed: {error}")),
                }
                self.network_status
                    .set_label(&format!("Added WireGuard network #{id}."));
                self.refresh().await;
            }
            Err(error) => self
                .network_status
                .set_label(&format!("Unable to add WireGuard network: {error}")),
        }
    }

    fn save_tor(&mut self, title: String, account: String, identity: String, note: String) {
        let record = account_store::new_record(
            AccountKind::Tor,
            daemon_api::normalized(
                &title,
                &format!("Tor {}", daemon_api::normalized(&identity, "linux")),
            ),
            Some("arti://local".to_owned()),
            daemon_api::normalized(&account, "default"),
            daemon_api::normalized(&identity, "linux"),
            None,
            None,
            Some(note),
        );
        match account_store::upsert(record) {
            Ok(accounts) => {
                self.account_status.set_label("Saved Tor account.");
                self.render_accounts(&accounts);
            }
            Err(error) => self
                .account_status
                .set_label(&format!("Unable to save Tor account: {error}")),
        }
    }

    async fn discover_tailnet(&mut self, email: String) {
        let Ok(email) = daemon_api::require_value(&email, "Email address") else {
            self.account_status
                .set_label("Enter an email address before Tailnet discovery.");
            return;
        };

        self.account_status.set_label("Finding Tailnet server...");
        match daemon_api::discover_tailnet(email).await {
            Ok(discovery) => {
                let kind = if discovery.managed {
                    "managed authority"
                } else {
                    "custom authority"
                };
                let issuer = discovery
                    .oidc_issuer
                    .map(|issuer| format!(" OIDC: {issuer}."))
                    .unwrap_or_default();
                self.account_status.set_label(&format!(
                    "Discovered {kind}: {}.{issuer}",
                    discovery.authority
                ));
            }
            Err(error) => self
                .account_status
                .set_label(&format!("Tailnet discovery failed: {error}")),
        }
    }

    async fn probe_tailnet(&mut self, authority: String) {
        let Ok(authority) = daemon_api::require_value(&authority, "Tailnet server URL") else {
            self.account_status
                .set_label("Enter a Tailnet server URL before checking it.");
            return;
        };

        self.account_status.set_label("Checking Tailnet server...");
        match daemon_api::probe_tailnet(authority).await {
            Ok(probe) => {
                let detail = probe
                    .detail
                    .unwrap_or_else(|| format!("HTTP {}", probe.status_code));
                self.account_status
                    .set_label(&format!("{}: {detail}", probe.summary));
            }
            Err(error) => self
                .account_status
                .set_label(&format!("Tailnet probe failed: {error}")),
        }
    }

    async fn start_tailnet_login(
        &mut self,
        authority: String,
        account: String,
        identity: String,
        hostname: Option<String>,
        sender: AsyncComponentSender<Self>,
    ) {
        let Ok(authority) = daemon_api::require_value(&authority, "Tailnet server URL") else {
            self.account_status
                .set_label("Enter a Tailnet server URL before sign-in.");
            return;
        };

        self.account_status.set_label("Starting Tailnet sign-in...");
        match daemon_api::start_tailnet_login(authority, account, identity, hostname).await {
            Ok(status) => {
                self.apply_login_status(&status);
                if let Some(auth_url) = status.auth_url.as_deref() {
                    if let Err(error) = open_auth_url(auth_url) {
                        self.account_status.set_label(&format!(
                            "{} Open this URL manually: {auth_url}. Browser launch failed: {error}",
                            self.account_status.text()
                        ));
                    }
                }
                if !status.running {
                    sender.input(HomeScreenMsg::PollTailnetLogin);
                }
            }
            Err(error) => self
                .account_status
                .set_label(&format!("Tailnet sign-in failed: {error}")),
        }
    }

    async fn poll_tailnet_login(&mut self, sender: AsyncComponentSender<Self>) {
        let Some(session_id) = self.tailnet_session_id.clone() else {
            return;
        };
        if self.tailnet_running {
            return;
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        match daemon_api::tailnet_login_status(session_id).await {
            Ok(status) => {
                self.apply_login_status(&status);
                if !status.running {
                    sender.input(HomeScreenMsg::PollTailnetLogin);
                }
            }
            Err(error) => {
                self.account_status
                    .set_label(&format!("Tailnet sign-in status failed: {error}"));
                self.tailnet_session_id = None;
            }
        }
    }

    async fn cancel_tailnet_login(&mut self) {
        let Some(session_id) = self.tailnet_session_id.clone() else {
            self.account_status
                .set_label("No Tailnet sign-in is active.");
            return;
        };
        match daemon_api::cancel_tailnet_login(session_id).await {
            Ok(()) => {
                self.tailnet_session_id = None;
                self.tailnet_running = false;
                self.account_status.set_label("Tailnet sign-in cancelled.");
            }
            Err(error) => self
                .account_status
                .set_label(&format!("Unable to cancel Tailnet sign-in: {error}")),
        }
    }

    async fn add_tailnet(
        &mut self,
        authority: String,
        account: String,
        identity: String,
        hostname: Option<String>,
        tailnet: Option<String>,
    ) {
        let Ok(authority) = daemon_api::require_value(&authority, "Tailnet server URL") else {
            self.account_status
                .set_label("Enter a Tailnet server URL before saving.");
            return;
        };
        if self.tailnet_session_id.is_some() && !self.tailnet_running {
            self.account_status
                .set_label("Finish browser sign-in before saving this Tailnet account.");
            return;
        }

        let stored_authority = daemon_api::normalized_optional(&authority)
            .unwrap_or_else(|| daemon_api::default_tailnet_authority().to_owned());
        let stored_account = daemon_api::normalized(&account, "default");
        let stored_identity = daemon_api::normalized(&identity, "linux");
        let stored_hostname = hostname.clone();
        let stored_tailnet = tailnet.clone();

        match daemon_api::add_tailnet(authority, account, identity, hostname, tailnet).await {
            Ok(id) => {
                let title = stored_tailnet
                    .clone()
                    .or(stored_hostname.clone())
                    .unwrap_or_else(|| format!("Tailnet {id}"));
                let record = account_store::new_record(
                    AccountKind::Tailnet,
                    title,
                    Some(stored_authority),
                    stored_account,
                    stored_identity,
                    stored_hostname,
                    stored_tailnet,
                    Some(format!("Linked to daemon network #{id}.")),
                );
                match account_store::upsert(record) {
                    Ok(accounts) => self.render_accounts(&accounts),
                    Err(error) => self
                        .account_status
                        .set_label(&format!("Tailnet account save failed: {error}")),
                }
                self.account_status
                    .set_label(&format!("Saved Tailnet account and network #{id}."));
                self.refresh().await;
            }
            Err(error) => self
                .account_status
                .set_label(&format!("Unable to save Tailnet account: {error}")),
        }
    }

    fn apply_login_status(&mut self, status: &daemon_api::TailnetLoginStatus) {
        self.tailnet_session_id = Some(status.session_id.clone());
        self.tailnet_running = status.running;

        let mut parts = Vec::new();
        if status.running {
            parts.push("Signed In".to_owned());
        } else if status.needs_login {
            parts.push("Browser Sign-In Required".to_owned());
        } else {
            parts.push("Checking Sign-In".to_owned());
        }
        if !status.backend_state.is_empty() {
            parts.push(format!("State: {}", status.backend_state));
        }
        if let Some(tailnet_name) = &status.tailnet_name {
            parts.push(format!("Tailnet: {tailnet_name}"));
        }
        if let Some(self_dns_name) = &status.self_dns_name {
            parts.push(self_dns_name.clone());
        }
        if !status.tailnet_ips.is_empty() {
            parts.push(status.tailnet_ips.join(", "));
        }
        if !status.health.is_empty() {
            parts.push(status.health.join(" / "));
        }
        self.account_status.set_label(&parts.join("\n"));
    }

    fn render_networks(&self, networks: &[daemon_api::NetworkSummary]) {
        while let Some(child) = self.network_cards.first_child() {
            self.network_cards.remove(&child);
        }

        if networks.is_empty() {
            self.network_cards.append(&empty_networks_view());
            return;
        }

        for network in networks {
            self.network_cards.append(&network_card(network));
        }
    }

    fn render_accounts(&self, accounts: &[AccountRecord]) {
        while let Some(child) = self.account_rows.first_child() {
            self.account_rows.remove(&child);
        }

        if accounts.is_empty() {
            self.account_rows.append(&empty_accounts_view());
            return;
        }

        for account in accounts {
            self.account_rows.append(&account_card(account));
        }
    }
}

fn configure_add_popover(button: &gtk::MenuButton, sender: &AsyncComponentSender<HomeScreen>) {
    let popover = gtk::Popover::new();
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
    box_.set_margin_all(6);

    for (label, msg) in [
        ("Add WireGuard Network", HomeScreenMsg::OpenWireGuard),
        ("Save Tor Account", HomeScreenMsg::OpenTor),
        ("Add Tailnet Account", HomeScreenMsg::OpenTailnet),
    ] {
        let item = gtk::Button::with_label(label);
        item.add_css_class("flat");
        item.set_halign(Align::Fill);
        let input = sender.input_sender().clone();
        item.connect_clicked(move |_| input.emit(msg_from_template(&msg)));
        box_.append(&item);
    }

    popover.set_child(Some(&box_));
    button.set_popover(Some(&popover));
}

fn msg_from_template(msg: &HomeScreenMsg) -> HomeScreenMsg {
    match msg {
        HomeScreenMsg::OpenWireGuard => HomeScreenMsg::OpenWireGuard,
        HomeScreenMsg::OpenTor => HomeScreenMsg::OpenTor,
        HomeScreenMsg::OpenTailnet => HomeScreenMsg::OpenTailnet,
        _ => unreachable!(),
    }
}

fn network_card(network: &daemon_api::NetworkSummary) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    card.add_css_class("network-card");
    if network.title.to_ascii_lowercase().contains("wireguard") {
        card.add_css_class("wireguard-card");
    } else {
        card.add_css_class("tailnet-card");
    }
    card.set_size_request(360, 175);
    card.set_margin_bottom(8);

    let kind = if network.title.to_ascii_lowercase().contains("wireguard") {
        "WireGuard"
    } else {
        "Tailnet"
    };
    let kind_label = gtk::Label::new(Some(kind));
    kind_label.add_css_class("network-card-kind");
    kind_label.set_xalign(0.0);

    let title = gtk::Label::new(Some(&network.title));
    title.add_css_class("network-card-title");
    title.set_xalign(0.0);
    title.set_wrap(true);

    let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    spacer.set_vexpand(true);

    let detail = gtk::Label::new(Some(&network.detail));
    detail.add_css_class("network-card-detail");
    detail.set_xalign(0.0);
    detail.set_wrap(true);
    detail.set_lines(4);

    card.append(&kind_label);
    card.append(&title);
    card.append(&spacer);
    card.append(&detail);
    card
}

fn empty_networks_view() -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 6);
    box_.add_css_class("empty-state");
    box_.set_size_request(520, 175);
    box_.set_hexpand(true);

    let title = gtk::Label::new(Some("No Networks Yet"));
    title.add_css_class("title-3");
    title.set_xalign(0.0);
    let detail = gtk::Label::new(Some(
        "Add a WireGuard network, or save a Tailnet account so Burrow can store a managed network when the daemon is reachable.",
    ));
    detail.add_css_class("dim-label");
    detail.set_wrap(true);
    detail.set_xalign(0.0);

    box_.append(&title);
    box_.append(&detail);
    box_
}

fn empty_accounts_view() -> gtk::Box {
    let box_ = gtk::Box::new(gtk::Orientation::Vertical, 6);
    box_.add_css_class("empty-state");
    box_.set_hexpand(true);

    let title = gtk::Label::new(Some("No Accounts Yet"));
    title.add_css_class("title-3");
    title.set_justify(gtk::Justification::Center);
    let detail = gtk::Label::new(Some(
        "Save a Tor account or sign in to Tailnet to keep network identities ready on this device.",
    ));
    detail.add_css_class("dim-label");
    detail.set_wrap(true);
    detail.set_justify(gtk::Justification::Center);

    box_.append(&title);
    box_.append(&detail);
    box_
}

fn account_card(account: &AccountRecord) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.add_css_class("summary-card");
    card.set_hexpand(true);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = gtk::Label::new(Some(&account.title));
    title.add_css_class("title-3");
    title.set_xalign(0.0);
    title.set_hexpand(true);
    let kind = gtk::Label::new(Some(account.kind.title()));
    kind.add_css_class("dim-label");
    header.append(&title);
    header.append(&kind);
    card.append(&header);

    append_account_value(&card, "Account", &account.account);
    append_account_value(&card, "Identity", &account.identity);
    if let Some(authority) = &account.authority {
        append_account_value(&card, "Authority", authority);
    }
    if let Some(hostname) = &account.hostname {
        append_account_value(&card, "Hostname", hostname);
    }
    if let Some(tailnet) = &account.tailnet {
        append_account_value(&card, "Tailnet", tailnet);
    }
    if let Some(note) = &account.note {
        let note_label = gtk::Label::new(Some(note));
        note_label.add_css_class("dim-label");
        note_label.set_wrap(true);
        note_label.set_xalign(0.0);
        card.append(&note_label);
    }

    card
}

fn append_account_value(card: &gtk::Box, label: &str, value: &str) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let key = gtk::Label::new(Some(label));
    key.add_css_class("dim-label");
    key.set_xalign(0.0);
    key.set_width_chars(9);
    let value = gtk::Label::new(Some(value));
    value.set_xalign(0.0);
    value.set_wrap(true);
    value.set_hexpand(true);
    row.append(&key);
    row.append(&value);
    card.append(&row);
}

fn open_wireguard_window(root: &gtk::ScrolledWindow, sender: &AsyncComponentSender<HomeScreen>) {
    let window = sheet_window(root, "WireGuard", 560, 620);
    let content = sheet_content(
        &window,
        "Import WireGuard",
        "Import a tunnel and optional account metadata.",
    );

    let title = gtk::Entry::new();
    title.set_placeholder_text(Some("Title"));
    let account = gtk::Entry::new();
    account.set_placeholder_text(Some("Account"));
    let identity = gtk::Entry::new();
    identity.set_placeholder_text(Some("Identity"));
    let text = gtk::TextView::new();
    text.set_monospace(true);
    text.set_wrap_mode(gtk::WrapMode::WordChar);

    let editor = gtk::ScrolledWindow::new();
    editor.set_min_content_height(220);
    editor.set_child(Some(&text));

    content.append(&section_label("Identity"));
    content.append(&title);
    content.append(&account);
    content.append(&identity);
    content.append(&section_label("WireGuard Configuration"));
    content.append(&editor);

    let add = gtk::Button::with_label("Add Network");
    add.add_css_class("suggested-action");
    let input = sender.input_sender().clone();
    let window_for_click = window.clone();
    add.connect_clicked(move |_| {
        input.emit(HomeScreenMsg::AddWireGuard {
            title: title.text().to_string(),
            account: account.text().to_string(),
            identity: identity.text().to_string(),
            config: text_view_text(&text),
        });
        window_for_click.close();
    });
    content.append(&add);

    window.set_child(Some(&content));
    window.present();
}

fn open_tor_window(root: &gtk::ScrolledWindow, sender: &AsyncComponentSender<HomeScreen>) {
    let window = sheet_window(root, "Tor", 520, 540);
    let content = sheet_content(
        &window,
        "Configure Tor",
        "Store Arti account and identity preferences.",
    );

    let title = entry_with_text("Title", "Default Tor");
    let account = entry_with_text("Account", "default");
    let identity = entry_with_text("Identity", "linux");
    let addresses = entry_with_text("Virtual Addresses", "100.64.0.2/32");
    let dns = entry_with_text("DNS Resolvers", "1.1.1.1, 1.0.0.1");
    let mtu = entry_with_text("MTU", "1400");
    let listen = entry_with_text("Transparent Listener", "127.0.0.1:9040");

    content.append(&section_label("Identity"));
    content.append(&title);
    content.append(&account);
    content.append(&identity);
    content.append(&section_label("Tor Preferences"));
    content.append(&addresses);
    content.append(&dns);
    content.append(&mtu);
    content.append(&listen);

    let save = gtk::Button::with_label("Save Account");
    save.add_css_class("suggested-action");
    let input = sender.input_sender().clone();
    let window_for_click = window.clone();
    save.connect_clicked(move |_| {
        let note = [
            format!(
                "Addresses: {}",
                normalized_entry(&addresses, "100.64.0.2/32")
            ),
            format!("DNS: {}", normalized_entry(&dns, "1.1.1.1, 1.0.0.1")),
            format!("MTU: {}", normalized_entry(&mtu, "1400")),
            format!("Listen: {}", normalized_entry(&listen, "127.0.0.1:9040")),
        ]
        .join(" - ");
        input.emit(HomeScreenMsg::SaveTor {
            title: normalized_entry(&title, "Default Tor"),
            account: normalized_entry(&account, "default"),
            identity: normalized_entry(&identity, "linux"),
            note,
        });
        window_for_click.close();
    });
    content.append(&save);

    window.set_child(Some(&content));
    window.present();
}

fn open_tailnet_window(root: &gtk::ScrolledWindow, sender: &AsyncComponentSender<HomeScreen>) {
    let window = sheet_window(root, "Tailnet", 560, 680);
    let content = sheet_content(
        &window,
        "Connect Tailnet",
        "Save Tailnet authority, identity defaults, and login material.",
    );

    let email = gtk::Entry::new();
    email.set_placeholder_text(Some("Email address"));
    let authority = entry_with_text("Server URL", daemon_api::default_tailnet_authority());
    let tailnet = gtk::Entry::new();
    tailnet.set_placeholder_text(Some("Tailnet"));
    let account = entry_with_text("Account", "default");
    let identity = entry_with_text("Identity", "linux");
    let hostname = entry_with_text("Hostname", &hostname_fallback());

    content.append(&section_label("Connection"));
    content.append(&email);
    content.append(&authority);
    content.append(&tailnet);
    content.append(&section_label("Identity"));
    content.append(&account);
    content.append(&identity);
    content.append(&hostname);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let discover = gtk::Button::with_label("Refresh Server Lookup");
    let probe = gtk::Button::with_label("Check Server");
    let sign_in = gtk::Button::with_label("Start Sign-In");
    actions.append(&discover);
    actions.append(&probe);
    actions.append(&sign_in);
    content.append(&section_label("Authentication"));
    content.append(&actions);

    let input = sender.input_sender().clone();
    let email_for_click = email.clone();
    discover.connect_clicked(move |_| {
        input.emit(HomeScreenMsg::DiscoverTailnet(
            email_for_click.text().to_string(),
        ));
    });

    let input = sender.input_sender().clone();
    let authority_for_probe = authority.clone();
    probe.connect_clicked(move |_| {
        input.emit(HomeScreenMsg::ProbeTailnet(
            authority_for_probe.text().to_string(),
        ));
    });

    let input = sender.input_sender().clone();
    let authority_for_login = authority.clone();
    let account_for_login = account.clone();
    let identity_for_login = identity.clone();
    let hostname_for_login = hostname.clone();
    sign_in.connect_clicked(move |_| {
        input.emit(HomeScreenMsg::StartTailnetLogin {
            authority: authority_for_login.text().to_string(),
            account: normalized_entry(&account_for_login, "default"),
            identity: normalized_entry(&identity_for_login, "linux"),
            hostname: daemon_api::normalized_optional(&hostname_for_login.text()),
        });
    });

    let save = gtk::Button::with_label("Save Account");
    save.add_css_class("suggested-action");
    let input = sender.input_sender().clone();
    let window_for_click = window.clone();
    save.connect_clicked(move |_| {
        input.emit(HomeScreenMsg::AddTailnet {
            authority: authority.text().to_string(),
            account: normalized_entry(&account, "default"),
            identity: normalized_entry(&identity, "linux"),
            hostname: daemon_api::normalized_optional(&hostname.text()),
            tailnet: daemon_api::normalized_optional(&tailnet.text()),
        });
        window_for_click.close();
    });

    let cancel = gtk::Button::with_label("Cancel Sign-In");
    let input = sender.input_sender().clone();
    cancel.connect_clicked(move |_| {
        input.emit(HomeScreenMsg::CancelTailnetLogin);
    });

    content.append(&save);
    content.append(&cancel);

    window.set_child(Some(&content));
    window.present();
}

fn sheet_window(root: &gtk::ScrolledWindow, title: &str, width: i32, height: i32) -> gtk::Window {
    let window = gtk::Window::builder()
        .title(title)
        .default_width(width)
        .default_height(height)
        .modal(true)
        .build();
    if let Some(root) = root.root() {
        if let Ok(parent) = root.downcast::<gtk::Window>() {
            window.set_transient_for(Some(&parent));
        }
    }
    window
}

fn sheet_content(window: &gtk::Window, title: &str, detail: &str) -> gtk::Box {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_all(18);

    let summary = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    summary.add_css_class("summary-card");

    let copy = gtk::Box::new(gtk::Orientation::Vertical, 4);
    copy.set_hexpand(true);

    let title_label = gtk::Label::new(Some(title));
    title_label.add_css_class("title-3");
    title_label.set_xalign(0.0);

    let detail_label = gtk::Label::new(Some(detail));
    detail_label.add_css_class("dim-label");
    detail_label.set_wrap(true);
    detail_label.set_xalign(0.0);

    copy.append(&title_label);
    copy.append(&detail_label);
    summary.append(&copy);

    let close = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text("Close")
        .valign(Align::Start)
        .build();
    close.add_css_class("flat");
    let window_for_click = window.clone();
    close.connect_clicked(move |_| window_for_click.close());
    summary.append(&close);

    content.append(&summary);
    content
}

fn section_label(label: &str) -> gtk::Label {
    let section = gtk::Label::new(Some(label));
    section.add_css_class("heading");
    section.set_xalign(0.0);
    section
}

fn entry_with_text(placeholder: &str, value: &str) -> gtk::Entry {
    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(placeholder));
    entry.set_text(value);
    entry
}

fn normalized_entry(entry: &gtk::Entry, fallback: &str) -> String {
    daemon_api::normalized(&entry.text(), fallback)
}

fn hostname_fallback() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "linux".to_owned())
}

fn text_view_text(text_view: &gtk::TextView) -> String {
    let buffer = text_view.buffer();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

fn open_auth_url(url: &str) -> anyhow::Result<()> {
    gtk::gio::AppInfo::launch_default_for_uri(url, None::<&gtk::gio::AppLaunchContext>)
        .map_err(anyhow::Error::from)
}
