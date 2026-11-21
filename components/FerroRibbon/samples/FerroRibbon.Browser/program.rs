#![cfg_attr(target_os = "emscripten", no_main)]

#[cfg(target_os = "emscripten")]
mod browser {
    use ferroui_base::utilities::{current_ui_locale, set_current_locale};
    use ferroui_base::{FerroObjectImpl, Ref, ferro_class, ferro_impl_classes, instantiate};
    use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions};
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, NewApplication,
    };
    use ferroui_fonts_inter::AppBuilderExtension;
    use std::rc::Rc;
    use wasm_bindgen::prelude::wasm_bindgen;

    #[repr(C)]
    struct BrowserApp {
        base: Application,
    }

    ferro_class!(BrowserApp: Application);
    ferro_impl_classes!(BrowserApp: FerroObjectImpl);

    impl NewApplication for BrowserApp {
        fn new_application() -> Ref<Self> {
            instantiate(Self {
                base: Application::construct(),
            })
        }
    }

    impl ApplicationImpl for BrowserApp {
        fn initialize(this: &Self) {
            ferroribbon_sample::initialize_gallery(this);
            set_current_locale(current_ui_locale());
        }

        fn on_framework_initialization_completed(this: &Self) {
            if let Some(lifetime) = this
                .application_lifetime()
                .as_ref()
                .and_then(|lifetime| lifetime.as_single_view_application_lifetime())
            {
                lifetime.set_main_view(Some(ferroribbon_sample::create_shell_view().upcast()));
            }
            Self::parent_on_framework_initialization_completed(this);
        }
    }

    #[wasm_bindgen(js_name = runMain)]
    pub fn run_main() {
        let mut options = ferroribbon_sample::gallery_font_options();
        options.default_family_name = Some("fonts:Inter#Inter".into());
        AppBuilder::configure::<BrowserApp>()
            .with_inter_font()
            .with(Rc::new(options))
            .start_browser_app("out", Some(BrowserPlatformOptions::default()));
    }
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    eprintln!("Build FerroRibbon Gallery with the wasm32-unknown-emscripten target.");
}
