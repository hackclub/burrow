use super::*;

pub struct SettingsScreen {
    _diag_group: AsyncController<settings::DiagGroup>,
}

pub struct SettingsScreenInit {
    pub daemon_client: Arc<Mutex<Option<DaemonClient>>>,
}

#[relm4::component(pub)]
impl SimpleComponent for SettingsScreen {
    type Init = SettingsScreenInit;
    type Input = ();
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
            .forward(sender.input_sender(), |_| ());

        let widgets = view_output!();
        widgets.preferences.add(diag_group.widget());

        let model = SettingsScreen {
            _diag_group: diag_group,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _: Self::Input, _sender: ComponentSender<Self>) {}
}
