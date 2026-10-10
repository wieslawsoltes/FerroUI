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

/// The render backend a module draws with.
///
/// Not in the original, which has one backend. The rendering modes
/// ([`BrowserRenderingMode`]) keep their meaning with either: they say what
/// the canvas of a view is (the canvas of a WebGL context, a 2D canvas),
/// and the backend draws to it with what it has for that kind of canvas.
///
/// | mode | Skia | Vello |
/// |---|---|---|
/// | `WebGL2` | Ganesh | the hybrid mode (`vello_gpu` over the WebGL2 context) |
/// | `WebGL1` | Ganesh | not drawn: the mode is taken out of the list |
/// | `Software2D` | Skia raster | the CPU mode (`vello_cpu`) |
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BrowserRenderer {
    /// The Skia backend.
    #[default]
    Skia,
    /// The Vello backend. The module has to be built with the feature
    /// `vello` of this crate.
    Vello,
}

impl BrowserRenderer {
    /// The backend with the given name, ignoring case.
    pub fn parse(name: &str) -> Option<BrowserRenderer> {
        [BrowserRenderer::Skia, BrowserRenderer::Vello].into_iter().find(|renderer| renderer.name().eq_ignore_ascii_case(name))
    }

    /// The name of the backend.
    pub fn name(self) -> &'static str {
        match self {
            BrowserRenderer::Skia => "Skia",
            BrowserRenderer::Vello => "Vello",
        }
    }

    /// Whether the module was built with the backend.
    pub fn is_available(self) -> bool {
        match self {
            BrowserRenderer::Skia => true,
            BrowserRenderer::Vello => cfg!(feature = "vello"),
        }
    }

    /// The rendering modes of `modes` the backend draws, in their order.
    /// The Vello backend has no renderer for WebGL 1; a list that asks for
    /// nothing else is drawn in memory.
    pub fn rendering_modes(self, modes: &[BrowserRenderingMode]) -> Vec<BrowserRenderingMode> {
        match self {
            BrowserRenderer::Skia => modes.to_vec(),
            BrowserRenderer::Vello => {
                let drawn: Vec<BrowserRenderingMode> =
                    modes.iter().copied().filter(|mode| *mode != BrowserRenderingMode::WebGL1).collect();
                match drawn.is_empty() && !modes.is_empty() {
                    true => vec![BrowserRenderingMode::Software2D],
                    false => drawn,
                }
            }
        }
    }
}

/// Options of the browser backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserPlatformOptions {
    /// The rendering modes with fallbacks. The first element has the
    /// highest priority.
    pub rendering_mode: Vec<BrowserRenderingMode>,

    /// The render backend. Skia by default.
    ///
    /// Not in the original. The Vello backend needs a module built with the
    /// feature `vello` of this crate; the application fails to start
    /// without it, with a message that names the feature.
    pub renderer: BrowserRenderer,

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
    /// default the scope is the directory of the worker script, which is the
    /// root of the site (the documentation of the original says the domain
    /// root, which differs when the site is served from a subpath).
    pub ferro_service_worker_scope: Option<String>,

    /// The file dialogs use the `native-file-system-adapter` polyfill. If
    /// the native implementation is available, by default it is used. This
    /// property forces the polyfill to be always used. For more details, see
    /// https://github.com/jimmywarting/native-file-system-adapter#a-note-when-downloading-with-the-polyfilled-version.
    pub prefer_file_dialog_polyfill: bool,

    /// Whether a module built with threads renders on a render thread: the
    /// compositor and the render backend then run in a worker that owns the
    /// canvases of the views, and the thread of the page keeps the user
    /// interface. `true` by default. With `false` such a module renders on
    /// the thread of the page, as a module built without threads always
    /// does; the option has no effect there.
    ///
    /// Not in the original, where a build with threads always renders on
    /// its render worker.
    pub render_thread: bool,
}

impl Default for BrowserPlatformOptions {
    fn default() -> Self {
        Self {
            rendering_mode: vec![
                BrowserRenderingMode::WebGL2,
                BrowserRenderingMode::WebGL1,
                BrowserRenderingMode::Software2D,
            ],
            renderer: BrowserRenderer::Skia,
            register_ferro_service_worker: false,
            ferro_service_worker_scope: None,
            prefer_file_dialog_polyfill: false,
            render_thread: true,
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

    let renderer = options.renderer;
    FerroLocator::current_mutable().bind_to_self(options);

    BrowserWindowingPlatform::set_global_this(dom_helper::get_global_this());

    let builder = if builder.windowing_subsystem_initializer().is_none() {
        builder.use_browser()
    } else {
        builder.clone()
    };

    match renderer {
        BrowserRenderer::Skia => builder,
        BrowserRenderer::Vello => use_vello_renderer(&builder),
    }
}

/// The builder with the Vello backend in the place of the render backend it
/// had: the last one chosen draws.
///
/// The modes of the backend are the hybrid mode and then the CPU mode: a
/// canvas with a WebGL2 context is drawn by the first, a 2D canvas and
/// everything that ends in memory (bitmaps, layers) by the second. Options
/// the application registered are kept.
#[cfg(feature = "vello")]
fn use_vello_renderer(builder: &AppBuilder) -> AppBuilder {
    use ferroui_vello::{VelloApplicationExtensions, VelloOptions, VelloRenderingMode};

    if FerroLocator::current().get_service::<VelloOptions>().is_none() {
        FerroLocator::current_mutable().bind_to_self(Rc::new(VelloOptions {
            rendering_modes: [Some(VelloRenderingMode::Hybrid), Some(VelloRenderingMode::Cpu), None],
            ..VelloOptions::default()
        }));
    }

    builder.use_vello()
}

/// # Panics
/// Always: the module has no Vello backend.
#[cfg(not(feature = "vello"))]
fn use_vello_renderer(_builder: &AppBuilder) -> AppBuilder {
    panic!(
        "BrowserPlatformOptions::renderer asks for the Vello backend, and the module was built without the feature \
         `vello` of ferroui-browser (scripts/build-browser.sh <application> --features vello; \
         docs/porting/vello-backend.md, section 12)"
    );
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
    fn a_module_with_threads_renders_on_a_render_thread_unless_told_not_to() {
        assert!(BrowserPlatformOptions::default().render_thread);
    }

    #[test]
    fn skia_is_the_default_renderer_and_draws_every_mode() {
        let options = BrowserPlatformOptions::default();

        assert_eq!(BrowserRenderer::Skia, options.renderer);
        assert!(BrowserRenderer::Skia.is_available());
        assert_eq!(options.rendering_mode, BrowserRenderer::Skia.rendering_modes(&options.rendering_mode));
    }

    #[test]
    fn renderers_are_parsed_by_name_ignoring_case() {
        assert_eq!(Some(BrowserRenderer::Vello), BrowserRenderer::parse("vello"));
        assert_eq!(Some(BrowserRenderer::Skia), BrowserRenderer::parse("SKIA"));
        assert_eq!(None, BrowserRenderer::parse("Direct2D"));
        assert_eq!(None, BrowserRenderer::parse(""));
        assert_eq!(cfg!(feature = "vello"), BrowserRenderer::Vello.is_available());
    }

    #[test]
    fn the_vello_renderer_draws_no_webgl1_canvas() {
        use BrowserRenderingMode::{Software2D, WebGL1, WebGL2};

        // The default order: the hybrid mode, then the CPU mode.
        assert_eq!(vec![WebGL2, Software2D], BrowserRenderer::Vello.rendering_modes(&[WebGL2, WebGL1, Software2D]));
        assert_eq!(vec![Software2D], BrowserRenderer::Vello.rendering_modes(&[Software2D]));
        assert_eq!(vec![WebGL2], BrowserRenderer::Vello.rendering_modes(&[WebGL2]));
        // WebGL 1 alone: in memory, which is what the script side falls
        // back to for a list it cannot serve.
        assert_eq!(vec![Software2D], BrowserRenderer::Vello.rendering_modes(&[WebGL1]));
        assert!(BrowserRenderer::Vello.rendering_modes(&[]).is_empty());
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
