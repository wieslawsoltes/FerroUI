//! A scene of the frame benchmarks in a desktop window, for the window suite of the local
//! performance gate (the GPU backends and the start of a real process).
//!
//! ```text
//! FRAMES_SCENE=text PERF_REPORT=1 cargo run --release -p ferroui-frame-benchmarks
//! ```
//!
//! `FRAMES_SCENE` names the scene (`scenes.rs`; default `text`); `PERF_FONTS` the folder of the
//! Roboto files (default: the Material theme's assets of this checkout). It prints `PERF first_frame`
//! on the first animation frame and, with `PERF_REPORT=1`, `PERF fps N` once a second.

use ferroui_base::media::FontManager;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_frame_benchmarks::{scenes, FrameHost, Roboto};
use ferroui_themes_fluent::FluentTheme;
use std::rc::Rc;

#[repr(C)]
struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_impl_classes!(App: FerroObjectImpl);

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self { base: Application::construct() })
    }
}

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.set_requested_theme_variant(Some(ThemeVariant::light()));
        this.styles().add(FluentTheme::new().as_style());
        FontManager::current().add_font_collection(Rc::new(Roboto::new()));
    }

    fn on_framework_initialization_completed(this: &Self) {
        let name = std::env::var("FRAMES_SCENE").unwrap_or_else(|_| "text".to_string());
        let scene = scenes::scene(&name).unwrap_or_else(|| panic!("no scene {name}"));
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            let window = Window::new();
            window.set_title(Some(format!("Frame benchmarks: {name}")));
            window.set_width(1280.0);
            window.set_height(820.0);
            window.set_can_resize(false);
            window.set_content(Some(Control::boxed(&FrameHost::new((scene.build)()))));
            desktop.set_main_window(Some(window));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let builder = AppBuilder::configure::<App>();
    #[cfg(all(target_os = "macos", feature = "quartz"))]
    let builder = builder.with(std::rc::Rc::new(ferroui_native::FerroNativePlatformOptions {
        rendering_mode: vec![if cfg!(feature = "quartz-metal") {
            ferroui_native::FerroNativeRenderingMode::Metal
        } else {
            ferroui_native::FerroNativeRenderingMode::Software
        }],
        ..Default::default()
    }));
    let exit_code = builder.use_platform_detect().start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(exit_code as u8)
}
