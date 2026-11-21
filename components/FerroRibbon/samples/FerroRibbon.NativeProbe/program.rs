use ferroribbon_native_probe::{layout_preview::SourceLayoutPreview, view_model::ProbeViewModel};
use ferroui_base::{FerroObjectImpl, Ref, ferro_class, ferro_impl_classes, instantiate};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, NewApplication,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_themes_simple::SimpleTheme;
use std::rc::Rc;
#[repr(C)]
struct App {
    base: Application,
}
ferro_class!(App:Application);
ferro_impl_classes!(App:FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        ferroribbon_native_probe::register_types();
        this.set_name(Some("FerroRibbon Native Probe".into()));
        this.styles()
            .add(SimpleTheme::new().upcast::<ferroui_base::styling::Styles>());
    }
    fn on_framework_initialization_completed(this: &Self) {
        if let Some(desktop) = this
            .application_lifetime()
            .as_ref()
            .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        {
            let model = ProbeViewModel::new(Rc::new(SourceLayoutPreview));
            let window = ferroribbon_native_probe::create_window(model);
            window
                .opened(|| println!("FerroRibbon native probe opened"))
                .detach();
            window
                .closed(|| println!("FerroRibbon native probe closed"))
                .detach();
            desktop.set_main_window(Some(window));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    AppBuilder::configure::<App>()
        .use_platform_detect()
        .start_with_classic_desktop_lifetime(&args);
}
