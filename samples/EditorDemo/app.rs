use crate::EditorDemo;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication, Window};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct App {
    base: Application,
    demo: RefCell<Option<Rc<EditorDemo>>>,
}
ferro_class!(App:Application);
ferro_impl_classes!(App:FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
            demo: RefCell::new(None),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        ferroui_markup_xaml::register_types();
        ferroui_edit::register_types();
        #[cfg(any(feature = "desktop", feature = "headless"))]
        {
            ferroui_themes_fluent::register_types();
            this.styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style());
        }
        #[cfg(not(any(feature = "desktop", feature = "headless")))]
        let _ = this;
    }
    fn on_framework_initialization_completed(this: &Self) {
        if let Some(desktop) = this
            .application_lifetime()
            .as_ref()
            .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        {
            let demo = EditorDemo::new(std::env::args().any(|argument| argument == "--dark"))
                .expect("Editor sample resources must load");
            let window = Window::new();
            window.set_title(Some("FerroUI Editor Demo".into()));
            window.set_width(1180.0);
            window.set_height(800.0);
            window.set_min_width(1000.0);
            window.set_min_height(480.0);
            window.set_content(Some(Rc::new(demo.view())));
            desktop.set_main_window(Some(window));
            if std::env::args().any(|argument| argument == "--smoke-actions") {
                crate::smoke::start(demo.clone());
            }
            *this.demo.borrow_mut() = Some(demo);
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
