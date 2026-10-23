//! Desktop application host; all gallery composition remains platform-independent.

use crate::GalleryView;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::{
    Application, ApplicationImpl, ApplicationImplExt, NewApplication, TextBlock, Window,
};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct App {
    base: Application,
    gallery: RefCell<Option<GalleryView>>,
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
        ferroui_markup_xaml::register_types();
        ferroui_themes_winui::register_types();
        this.styles()
            .add(ferroui_themes_winui::WinUITheme::new().as_style());
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime
            .as_ref()
            .and_then(|lifetime| lifetime.as_classic_desktop_style_application_lifetime())
        {
            let window = Window::new();
            window.set_title(Some("WinUI Gallery · FerroUI".to_string()));
            window.set_width(1280.0);
            window.set_height(900.0);
            window.set_min_width(720.0);
            window.set_min_height(480.0);
            match GalleryView::with_preview_options(
                ferroui_markup_xaml_loader::winui::WinUIXamlOptions::for_winui_theme(),
            ) {
                Ok(gallery) => {
                    if let Some(page) = std::env::args()
                        .skip(1)
                        .find_map(|arg| arg.strip_prefix("--page=").map(str::to_string))
                    {
                        gallery.navigate(&page);
                    }
                    window.set_content(Some(Rc::new(gallery.root())));
                    *this.gallery.borrow_mut() = Some(gallery);
                }
                Err(error) => {
                    let error_text = TextBlock::new();
                    error_text.set_text(Some(&format!("Gallery initialization failed: {error}")));
                    window.set_content(Some(Rc::new(error_text)));
                }
            }
            desktop.set_main_window(Some(window));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

impl App {
    pub fn construct() -> Self {
        Self {
            base: Application::construct(),
            gallery: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
