use gtk::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/hackclub/burrow/window.ui")]
    pub struct BurrowGtkWindow {
        // Template widgets
        #[template_child]
        pub header_bar: TemplateChild<gtk::HeaderBar>,
        #[template_child]
        pub label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BurrowGtkWindow {
        const NAME: &'static str = "BurrowGtkWindow";
        type Type = super::BurrowGtkWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BurrowGtkWindow {}
    impl WidgetImpl for BurrowGtkWindow {}
    impl WindowImpl for BurrowGtkWindow {}
    impl ApplicationWindowImpl for BurrowGtkWindow {}
    impl AdwApplicationWindowImpl for BurrowGtkWindow {}
}

glib::wrapper! {
    pub struct BurrowGtkWindow(ObjectSubclass<imp::BurrowGtkWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,        @implements gio::ActionGroup, gio::ActionMap;
}

impl BurrowGtkWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}
