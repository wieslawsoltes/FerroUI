use crate::browser_runtime_platform::BrowserRuntimePlatformServices;
use crate::browser_single_view_lifetime::BrowserSingleViewLifetime;
use crate::ferro_view::FerroView;
use crate::interop::dom_helper;
use crate::windowing_platform::BrowserWindowingPlatform;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::application_lifetimes::IApplicationLifetime;
use ferroui_controls::AppBuilder;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_skia::SkiaApplicationExtensions;
use std::rc::Rc;

/// How a view is rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BrowserRenderingMode {
    /// Rendered in memory and copied to a 2D canvas.
    Software2D = 1,
    /// Rendered with WebGL 1.
    WebGL1,
    /// Rendered with WebGL 2.
    WebGL2,
}

impl BrowserRenderingMode {
    /// The mode with the given name, ignoring case.
    pub fn parse(name: &str) -> Option<BrowserRenderingMode> {
        [BrowserRenderingMode::Software2D, BrowserRenderingMode::WebGL1, BrowserRenderingMode::WebGL2]
            .into_iter()
            .find(|mode| mode.name().eq_ignore_ascii_case(name))
    }

    /// The name of the mode.
    pub fn name(self) -> &'static str {
        match self {
            BrowserRenderingMode::Software2D => "Software2D",
            BrowserRenderingMode::WebGL1 => "WebGL1",
            BrowserRenderingMode::WebGL2 => "WebGL2",
        }
    }
}

/// Options of the browser backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserPlatformOptions {
    /// The rendering modes with fallbacks. The first element has the
    /// highest priority.
    pub rendering_mode: Vec<BrowserRenderingMode>,

    /// Defines if the service worker used by FerroUI should be registered.
    /// If registered, service worker can work as a save file picker fallback
    /// on the browsers that don't support native implementation. For more
    /// details, see
    /// https://github.com/jimmywarting/native-file-system-adapter#a-note-when-downloading-with-the-polyfilled-version.
    ///
    /// Unstable: this property might not work reliably.
    pub register_ferro_service_worker: bool,

    /// If [`register_ferro_service_worker`](Self::register_ferro_service_worker)
    /// is enabled, it is possible to redefine scope for the worker. By
    /// default, current domain root is used as a scope.
    pub ferro_service_worker_scope: Option<String>,

    /// The file dialogs use the `native-file-system-adapter` polyfill. If
    /// the native implementation is available, by default it is used. This
    /// property forces the polyfill to be always used. For more details, see
    /// https://github.com/jimmywarting/native-file-system-adapter#a-note-when-downloading-with-the-polyfilled-version.
    pub prefer_file_dialog_polyfill: bool,
}

impl Default for BrowserPlatformOptions {
    fn default() -> Self {
        Self {
            rendering_mode: vec![
                BrowserRenderingMode::WebGL2,
                BrowserRenderingMode::WebGL1,
                BrowserRenderingMode::Software2D,
            ],
            register_ferro_service_worker: false,
            ferro_service_worker_scope: None,
            prefer_file_dialog_polyfill: false,
        }
    }
}

/// Starts an application in a web page.
pub trait BrowserAppBuilder {
    /// Configures browser backend and creates a single view lifetime from
    /// the passed `main_div_id` parameter.
    ///
    /// `main_div_id` is the ID of the html element where the content should
    /// be rendered. The function returns once the application is set up;
    /// the page drives it from then on.
    fn start_browser_app(&self, main_div_id: &str, options: Option<BrowserPlatformOptions>);

    /// Configures browser backend.
    ///
    /// This method doesn't create any views to be rendered. To do so create
    /// a [`FerroView`] object. Alternatively, you can call
    /// [`start_browser_app`](Self::start_browser_app) instead.
    fn setup_browser_app(&self, options: Option<BrowserPlatformOptions>);

    /// Selects the subsystems of the browser: its runtime platform and
    /// windowing subsystem, Skia for rendering and HarfBuzz for text
    /// shaping.
    fn use_browser(&self) -> AppBuilder;
}

fn pre_setup_browser(builder: &AppBuilder, options: Option<BrowserPlatformOptions>) -> AppBuilder {
    let options = match options {
        Some(options) => Rc::new(options),
        None => FerroLocator::current().get_service::<BrowserPlatformOptions>().unwrap_or_default(),
    };

    FerroLocator::current_mutable().bind_to_self(options);

    BrowserWindowingPlatform::set_global_this(dom_helper::get_global_this());

    if builder.windowing_subsystem_initializer().is_none() {
        builder.use_browser()
    } else {
        builder.clone()
    }
}

impl BrowserAppBuilder for AppBuilder {
    fn start_browser_app(&self, main_div_id: &str, options: Option<BrowserPlatformOptions>) {
        let builder = pre_setup_browser(self, options);

        let lifetime = BrowserSingleViewLifetime::new();
        builder.after_application_setup({
            let lifetime = lifetime.clone();
            let main_div_id = main_div_id.to_string();
            move |_| {
                lifetime.set_view(Some(FerroView::new(&main_div_id)));
            }
        });

        let lifetime: Rc<dyn IApplicationLifetime> = lifetime;
        builder.setup_with_lifetime(lifetime);
    }

    fn setup_browser_app(&self, options: Option<BrowserPlatformOptions>) {
        let builder = pre_setup_browser(self, options);

        let lifetime: Rc<dyn IApplicationLifetime> = BrowserSingleViewLifetime::new();
        builder.setup_with_lifetime(lifetime);
    }

    fn use_browser(&self) -> AppBuilder {
        BrowserRuntimePlatformServices::use_browser_runtime_platform_subsystem(self)
            .use_windowing_subsystem(BrowserWindowingPlatform::register, "")
            .use_skia()
            .use_harfbuzz()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modes_have_the_values_the_script_side_uses() {
        assert_eq!(1, BrowserRenderingMode::Software2D as i32);
        assert_eq!(2, BrowserRenderingMode::WebGL1 as i32);
        assert_eq!(3, BrowserRenderingMode::WebGL2 as i32);
    }

    #[test]
    fn the_default_modes_fall_back_from_webgl2_to_software() {
        assert_eq!(
            vec![BrowserRenderingMode::WebGL2, BrowserRenderingMode::WebGL1, BrowserRenderingMode::Software2D],
            BrowserPlatformOptions::default().rendering_mode
        );
    }

    #[test]
    fn modes_are_parsed_by_name_ignoring_case() {
        assert_eq!(Some(BrowserRenderingMode::WebGL2), BrowserRenderingMode::parse("WebGL2"));
        assert_eq!(Some(BrowserRenderingMode::WebGL1), BrowserRenderingMode::parse("webgl1"));
        assert_eq!(Some(BrowserRenderingMode::Software2D), BrowserRenderingMode::parse("SOFTWARE2D"));
        assert_eq!(None, BrowserRenderingMode::parse("WebGPU"));
        assert_eq!(None, BrowserRenderingMode::parse(""));
    }
}
