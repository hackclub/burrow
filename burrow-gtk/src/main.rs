use adw::prelude::*;
use burrow::{DaemonClient, DaemonCommand, DaemonStartOptions};
use gtk::Align;
use relm4::{
    component::{AsyncComponent, AsyncComponentParts, AsyncComponentSender},
    prelude::*,
};

struct App {}

#[derive(Debug)]
enum Msg {
    Start,
    Stop,
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = Msg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Window {
            set_title: Some("Simple app"),
            set_default_size: (640, 480),

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,
                set_valign: Align::Center,

                gtk::Label {
                    set_label: "Burrow GTK Switch",
                },

                gtk::Switch {
                    set_halign: Align::Center,
                    set_hexpand: false,
                    set_vexpand: false,
                    connect_active_notify => move |switch|
                        sender.input(if switch.is_active() { Msg::Start } else { Msg::Stop })
                },
            }
        }
    }

    async fn init(
        _: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = App {};

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            Msg::Start => {
                let mut client = DaemonClient::new().await.unwrap();
                client
                    .send_command(DaemonCommand::Start(DaemonStartOptions::default()))
                    .await
                    .unwrap();
            }
            Msg::Stop => {
                let mut client = DaemonClient::new().await.unwrap();
                client.send_command(DaemonCommand::Stop).await.unwrap();
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("com.hackclub.burrow");
    app.run_async::<App>(());
}
