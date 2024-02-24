use super::*;

pub struct SettingsScreen {
    diag_group: AsyncController<settings::DiagGroup>,
    daemon_group: AsyncController<settings::DaemonGroup>,
}

pub struct SettingsScreenInit {
    pub daemon_client: Arc<Mutex<Option<DaemonClient>>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SettingsScreenMsg {
    DaemonStateChange,
}

#[relm4::component(pub)]
impl SimpleComponent for SettingsScreen {
    type Init = SettingsScreenInit;
    type Input = SettingsScreenMsg;
    type Output = ();

    view! {
        #[name(preferences)]
        adw::PreferencesPage {}
    }

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let diag_group = settings::DiagGroup::builder()
            .launch(settings::DiagGroupInit {
                daemon_client: Arc::clone(&init.daemon_client),
            })
            .forward(sender.input_sender(), |_| {
                SettingsScreenMsg::DaemonStateChange
            });

        let daemon_group = settings::DaemonGroup::builder()
            .launch(settings::DaemonGroupInit {
                daemon_client: Arc::clone(&init.daemon_client),
            })
            .forward(sender.input_sender(), |_| {
                SettingsScreenMsg::DaemonStateChange
            });

        let widgets = view_output!();
        widgets.preferences.add(diag_group.widget());
        widgets.preferences.add(daemon_group.widget());

        let model = SettingsScreen { diag_group, daemon_group };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _: Self::Input, _sender: ComponentSender<Self>) {
        //  Currently, `SettingsScreenMsg` only has one variant, so the if is ambiguous.
        //
        // if let SettingsScreenMsg::DaemonStateChange = msg {
        self.diag_group.emit(DiagGroupMsg::Refresh);
        self.daemon_group.emit(DaemonGroupMsg::DaemonStateChange);
        // }
    }
}
