pub struct Template {}

pub struct TemplateInit {}

#[derive(Debug)]
pub enum TemplateMsg {}

#[relm4::component(pub, async)]
impl AsyncComponent for Template {
    type Init = TemplateInit;
    type Input = TemplateMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Box {
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let widgets = view_output!();

        let model = Template {};

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
    }
}
