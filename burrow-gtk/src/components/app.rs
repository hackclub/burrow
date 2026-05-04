use super::*;
use anyhow::Context;

pub struct App {
    _home_screen: AsyncController<home_screen::HomeScreen>,
}

#[derive(Debug)]
pub enum AppMsg {
    None,
}

impl App {
    pub fn run() {
        let app = RelmApp::new(config::ID);
        relm4::set_global_css(APP_CSS);
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
            set_default_size: (900, 760),
        }
    }

    async fn init(
        _: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let home_screen = home_screen::HomeScreen::builder()
            .launch(())
            .forward(sender.input_sender(), |_| AppMsg::None);

        let widgets = view_output!();

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(
            &adw::HeaderBar::builder()
                .title_widget(&gtk::Label::new(Some("Burrow")))
                .build(),
        );
        content.append(home_screen.widget());

        root.set_content(Some(&content));

        let model = App { _home_screen: home_screen };

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppMsg::None => {}
        }
    }
}

const APP_CSS: &str = r#"
.empty-state {
  border-radius: 18px;
  padding: 22px;
  background: alpha(@card_bg_color, 0.72);
}

.summary-card {
  border-radius: 18px;
  padding: 14px;
  background: alpha(@card_bg_color, 0.72);
}

.network-card {
  border-radius: 10px;
  padding: 16px;
  box-shadow: 0 2px 6px alpha(black, 0.14);
}

.wireguard-card {
  background: linear-gradient(135deg, #3277d8, #174ea6);
}

.tailnet-card {
  background: linear-gradient(135deg, #31b891, #147d69);
}

.network-card-kind,
.network-card-title,
.network-card-detail {
  color: white;
}

.network-card-kind {
  opacity: 0.86;
  font-weight: 700;
}

.network-card-title {
  font-size: 1.22em;
  font-weight: 700;
}

.network-card-detail {
  opacity: 0.92;
  font-family: monospace;
}
"#;
