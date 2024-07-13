use super::*;
use anyhow::Context;
use std::time::Duration;

const RECONNECT_POLL_TIME: Duration = Duration::from_secs(5);

pub struct App {
    daemon_client: Arc<Mutex<Option<Channel>>>,
    settings_screen: Controller<settings_screen::SettingsScreen>,
    switch_screen: AsyncController<switch_screen::SwitchScreen>,
}

#[derive(Debug)]
pub enum AppMsg {
    None,
    PostInit,
}

impl App {
    pub fn run() {
        let app = RelmApp::new(config::ID);
        Self::setup_gresources().unwrap();
        Self::setup_i18n().unwrap();

        app.run_async::<App>(());
    }

    fn setup_i18n() -> Result<()> {
        gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "");
        gettextrs::bindtextdomain(config::GETTEXT_PACKAGE, config::LOCALEDIR)?;
        gettextrs::bind_textdomain_codeset(config::GETTEXT_PACKAGE, "UTF-8")?;
        gettextrs::textdomain(config::GETTEXT_PACKAGE)?;
        Ok(())
    }

    fn setup_gresources() -> Result<()> {
        gtk::gio::resources_register_include!("compiled.gresource")
            .context("Failed to register and include compiled gresource.")
    }
}

#[relm4::component(pub, async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Window {
            set_title: Some("Burrow"),
            set_default_size: (640, 480),
        }
    }

    async fn init(
        _: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        // TODO: RPC REFACTOR (Handle Error)
        let daemon_client = Arc::new(Mutex::new(Some(daemon::daemon_connect().await.unwrap())));

        let switch_screen = switch_screen::SwitchScreen::builder()
            .launch(switch_screen::SwitchScreenInit {
                daemon_client: Arc::clone(&daemon_client),
            })
            .forward(sender.input_sender(), |_| AppMsg::None);

        let settings_screen = settings_screen::SettingsScreen::builder()
            .launch(settings_screen::SettingsScreenInit {
                daemon_client: Arc::clone(&daemon_client),
            })
            .forward(sender.input_sender(), |_| AppMsg::None);

        let widgets = view_output!();

        let view_stack = adw::ViewStack::new();
        view_stack.add_titled(switch_screen.widget(), None, "Switch");
        view_stack.add_titled(settings_screen.widget(), None, "Settings");

        let view_switcher_bar = adw::ViewSwitcherBar::builder().stack(&view_stack).build();
        view_switcher_bar.set_reveal(true);

        //  When libadwaita 1.4 support becomes more avaliable, this approach is more appropriate
        //
        // let toolbar = adw::ToolbarView::new();
        // toolbar.add_top_bar(
        //     &adw::HeaderBar::builder()
        //         .title_widget(&gtk::Label::new(Some("Burrow")))
        //         .build(),
        // );
        // toolbar.add_bottom_bar(&view_switcher_bar);
        // toolbar.set_content(Some(&view_stack));
        // root.set_content(Some(&toolbar));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(
            &adw::HeaderBar::builder()
                .title_widget(&gtk::Label::new(Some("Burrow")))
                .build(),
        );
        content.append(&view_stack);
        content.append(&view_switcher_bar);

        root.set_content(Some(&content));

        sender.input(AppMsg::PostInit);

        let model = App {
            daemon_client,
            switch_screen,
            settings_screen,
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        _msg: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        loop {
            tokio::time::sleep(RECONNECT_POLL_TIME).await;
            {
                let mut daemon_client = self.daemon_client.lock().await;
                let mut disconnected_daemon_client = false;

                if let Some(daemon_client) = daemon_client.as_mut() {
                    let mut client = tunnel_client::TunnelClient::new(daemon_client);
                    if let Err(_e) = client.tunnel_status(burrow_rpc::Empty {}).await {
                        disconnected_daemon_client = true;
                        self.switch_screen
                            .emit(switch_screen::SwitchScreenMsg::DaemonDisconnect);
                        self.settings_screen
                            .emit(settings_screen::SettingsScreenMsg::DaemonStateChange)
                    }
                }

                if disconnected_daemon_client || daemon_client.is_none() {
                    match daemon::daemon_connect().await {
                        Ok(new_daemon_client) => {
                            *daemon_client = Some(new_daemon_client);
                            self.switch_screen
                                .emit(switch_screen::SwitchScreenMsg::DaemonReconnect);
                            self.settings_screen
                                .emit(settings_screen::SettingsScreenMsg::DaemonStateChange)
                        }
                        Err(_e) => {
                            //  TODO: Handle Error
                        }
                    }
                }
            }
        }
    }
}
