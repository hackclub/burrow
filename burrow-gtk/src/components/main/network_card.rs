use super::*;

pub struct NetworkCard {}

pub struct NetworkCardInit {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug)]
pub enum NetworkCardMsg {}

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
            }
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let widgets = view_output!();

        let model = NetworkCard {};

        AsyncComponentParts { model, widgets }
    }
}
