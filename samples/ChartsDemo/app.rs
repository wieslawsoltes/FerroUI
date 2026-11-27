use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::{
    Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication, Window,
};
use ferroui_themes_winui::WinUITheme;

#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_class_info!(App { new: App::new });
ferro_impl_classes!(App: FerroObjectImpl);

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        crate::register_types();
        // The units of the theme the sample needs: the table the build wrote (`build.rs`).
        this.styles().add(
            WinUITheme::with_parts(None, crate::compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME)
                .as_style(),
        );
    }

    fn on_framework_initialization_completed(this: &Self) {
        if let Some(desktop) = this
            .application_lifetime()
            .as_ref()
            .and_then(|lifetime| lifetime.as_classic_desktop_style_application_lifetime())
        {
            let window = Window::new();
            window.set_title(Some("FerroUI Charts".to_owned()));
            window.set_width(1120.0);
            window.set_height(880.0);
            window.set_content(Some(Control::boxed(crate::create_view())));
            desktop.set_main_window(Some(window));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

impl App {
    pub fn construct() -> Self {
        Self {
            base: Application::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
