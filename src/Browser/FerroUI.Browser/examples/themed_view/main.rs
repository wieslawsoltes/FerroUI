//! A view of themed controls in a web page: the end-to-end check of the
//! browser platform.
//!
//! The application adds the Fluent theme to its styles and shows, as the
//! main view of the single-view lifetime, a stack panel of a text block, a
//! button, a check box, a text box, a slider, a progress bar, a list box
//! that is too small for its items, so that it shows a scroll bar, and a
//! target that accepts text dropped on it. It is the page counterpart of the
//! `themed_window` example of the themes. On request of the behaviour tests
//! (`themedViewNativeHost`) it also shows a native control host, whose
//! native control is an element of the page.
//!
//! Build and assemble the site with `scripts/build-browser.sh themed_view`
//! and serve `target/browser/themed_view` with any static web server. The
//! query string selects the rendering mode and the theme variant:
//! `?RenderingMode=Software2D`, `?ThemeVariant=dark`. `?Renderer=Vello`
//! draws with the Vello backend in a module built with the feature `vello`
//! of the crate (the hybrid mode for `WebGL2`, the CPU mode for
//! `Software2D`); Skia draws otherwise.
//!
//! Built with threads (`scripts/build-browser.sh themed_view --threads`,
//! served cross-origin isolated) the view is rendered by a render thread:
//! the compositor and Skia run in a worker that owns the canvas.
//! `?RenderThread=false` keeps such a module on the thread of the page, for
//! comparison. `themedViewRendering` reports which thread drew the frames.
//! `themedViewSecondView` opens and closes a second view in another element
//! of the page, for the test of the disposal of a view.

#![cfg_attr(target_os = "emscripten", no_main)]

use ferroui_base::input::platform::ClipboardExtensions;
use ferroui_base::input::{DataTransferExtensions, DragDrop, DragDropEffects};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, JpegBitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, FontManagerOptions};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroLocator, FerroObjectImpl, LocatorExtensions,
    PixelSize, Ref, Thickness, Vector,
};
use ferroui_browser::interop::canvas_helper::{RENDER_TARGET_KIND_SOFTWARE, RENDER_TARGET_KIND_WEB_GL};
use ferroui_browser::interop::{navigation_helper, thread_proxy};
use ferroui_browser::rendering::{BrowserSharedRenderLoop, RenderStatistics, RenderWorker};
use ferroui_browser::{BrowserAppBuilder, BrowserPlatformOptions, BrowserRenderer, BrowserRenderingMode, FerroView};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Button, CheckBox, Control, ListBox,
    NativeControlHost, NewApplication, ProgressBar, Slider, StackPanel, TextBlock, TextBox, TopLevel,
};
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_themes_fluent::FluentTheme;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// The controls of the view whose state the behaviour tests read.
struct Controls {
    button: Ref<Button>,
    check_box: Ref<CheckBox>,
    text_box: Ref<TextBox>,
    slider: Ref<Slider>,
    list_box: Ref<ListBox>,
    drop_target: Ref<Border>,
    panel: Ref<StackPanel>,
    native_host: Ref<NativeControlHost>,
    clicks: Rc<Cell<u32>>,
}

/// What the services of the platform answered, for the behaviour tests.
#[derive(Default)]
struct Services {
    dropped: Option<String>,
    drag_overs: u32,
    clipboard: Option<String>,
    screen_details: Option<bool>,
    launched: Option<bool>,
    back_requests: u32,
    safe_area_changes: u32,
}

thread_local! {
    /// The second view of the page, while it is open (`themedViewSecondView`).
    static SECOND_VIEW: RefCell<Option<Rc<FerroView>>> = const { RefCell::new(None) };
    static THEME_VARIANT: RefCell<Option<String>> = const { RefCell::new(None) };
    static CONTROLS: RefCell<Option<Controls>> = const { RefCell::new(None) };
    static SERVICES: RefCell<Services> = RefCell::new(Services::default());
}

fn with_services(f: impl FnOnce(&mut Services)) {
    SERVICES.with(|services| f(&mut services.borrow_mut()));
}

#[repr(C)]
pub struct App {
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
        this.set_name(Some("FerroUI themed view".to_string()));
        match THEME_VARIANT.with(|variant| variant.borrow().clone()).as_deref() {
            Some("dark") => this.set_requested_theme_variant(Some(ThemeVariant::dark())),
            Some("light") => this.set_requested_theme_variant(Some(ThemeVariant::light())),
            _ => {}
        }
        this.styles().add(FluentTheme::new().as_style());
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(single_view) = lifetime.as_ref().and_then(|lifetime| lifetime.as_single_view_application_lifetime())
        {
            single_view.set_main_view(Some(create_main_view()));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

fn text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}

fn create_main_view() -> Ref<Control> {
    let title = TextBlock::new();
    title.set_text(Some("Fluent theme"));
    title.set_font_size(20.0);

    let button = Button::new();
    button.set_content(text("Button"));
    let clicks = Rc::new(Cell::new(0));
    button.click({
        let clicks = clicks.clone();
        move |_, _| clicks.set(clicks.get() + 1)
    });

    let check_box = CheckBox::new();
    check_box.set_content(text("Check box"));
    check_box.set_is_checked(Some(true));

    let text_box = TextBox::new();
    text_box.set_text(Some("Text box"));

    let slider = Slider::new();
    slider.set_minimum(0.0);
    slider.set_maximum(100.0);
    slider.set_range_value(40.0);

    let progress_bar = ProgressBar::new();
    progress_bar.set_minimum(0.0);
    progress_bar.set_maximum(100.0);
    progress_bar.set_range_value(65.0);
    progress_bar.set_height(16.0);

    let list_box = ListBox::new();
    for item in ["First item", "Second item", "Third item", "Fourth item", "Fifth item", "Sixth item", "Seventh item"] {
        list_box.items().add(text(item));
    }
    list_box.set_selected_index(1);
    // Smaller than its items: the scroll viewer of the list shows its scroll bar.
    list_box.set_height(96.0);

    let drop_label = TextBlock::new();
    drop_label.set_text(Some("Drop text here"));
    let drop_target = Border::new();
    drop_target.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(0xdd, 0xe6, 0xf5)))));
    drop_target.set_padding(Thickness::uniform(8.0));
    drop_target.set_width(160.0);
    drop_target.set_height(36.0);
    drop_target.set_horizontal_alignment(HorizontalAlignment::Left);
    drop_target.set_child(Some(drop_label.upcast()));
    DragDrop::set_allow_drop(&drop_target, true);
    DragDrop::add_drag_over_handler(&drop_target, |_, e| {
        with_services(|services| services.drag_overs += 1);
        e.set_drag_effects(e.drag_effects() & DragDropEffects::COPY);
    });
    DragDrop::add_drop_handler(&drop_target, |_, e| {
        let text = e.data_transfer().try_get_text();
        with_services(|services| services.dropped = text);
        e.set_drag_effects(e.drag_effects() & DragDropEffects::COPY);
    });

    let panel = StackPanel::new();
    panel.set_spacing(10.0);
    panel.set_margin(Thickness::uniform(16.0));
    panel.children().add(title);
    panel.children().add(button.clone());
    panel.children().add(check_box.clone());
    panel.children().add(text_box.clone());
    panel.children().add(slider.clone());
    panel.children().add(progress_bar);
    panel.children().add(list_box.clone());
    panel.children().add(drop_target.clone());

    CONTROLS.with(|controls| {
        *controls.borrow_mut() = Some(Controls {
            button: button.clone(),
            check_box: check_box.clone(),
            text_box: text_box.clone(),
            slider: slider.clone(),
            list_box: list_box.clone(),
            drop_target: drop_target.clone(),
            panel: panel.clone(),
            native_host: NativeControlHost::new(),
            clicks,
        })
    });

    panel.upcast()
}

/// The state of the controls as a line of `name=value` pairs, for the
/// behaviour tests that drive the page (`scripts/browser/tests`).
#[wasm_bindgen(js_name = themedViewState)]
pub fn themed_view_state() -> String {
    CONTROLS.with(|controls| {
        let controls = controls.borrow();
        let Some(c) = controls.as_ref() else { return String::new() };
        let focus = if c.text_box.is_focused() {
            "text_box"
        } else if c.button.is_focused() {
            "button"
        } else if c.check_box.is_focused() {
            "check_box"
        } else if c.slider.is_focused() {
            "slider"
        } else if c.list_box.is_focused() {
            "list_box"
        } else {
            "other"
        };
        format!(
            "clicks={};checked={:?};text={};caret={};slider={};selected={};focus={};button_over={}",
            c.clicks.get(),
            c.check_box.is_checked(),
            c.text_box.text().unwrap_or_default(),
            c.text_box.caret_index(),
            c.slider.value(),
            c.list_box.selected_index(),
            focus,
            c.button.is_pointer_over(),
        )
    })
}

/// The top-level of the view.
fn top_level() -> Option<Ref<TopLevel>> {
    CONTROLS.with(|controls| {
        controls.borrow().as_ref().and_then(|c| TopLevel::get_top_level(Some(&c.button)))
    })
}

/// What the services of the platform report, as a line of `name=value`
/// pairs: the screens, the safe area, the drop target and the answers of
/// the asynchronous services started by the functions below.
#[wasm_bindgen(js_name = themedViewServices)]
pub fn themed_view_services() -> String {
    let Some(top_level) = top_level() else { return String::new() };
    let mut line = Vec::new();

    if let Some(screens) = top_level.screens() {
        line.push(format!("screens={}", screens.screen_count()));
        if let Some(screen) = screens.screen_from_top_level(&top_level) {
            let bounds = screen.bounds();
            let working_area = screen.working_area();
            line.push(format!("bounds={},{},{},{}", bounds.x, bounds.y, bounds.width, bounds.height));
            line.push(format!(
                "working_area={},{},{},{}",
                working_area.x, working_area.y, working_area.width, working_area.height
            ));
            line.push(format!("scaling={}", screen.scaling()));
            line.push(format!("primary={}", screen.is_primary()));
            line.push(format!("orientation={:?}", screen.current_orientation()));
        }
    }

    if let Some(insets) = top_level.insets_manager() {
        let padding = insets.safe_area_padding();
        line.push(format!("safe_area={},{},{},{}", padding.left, padding.top, padding.right, padding.bottom));
        line.push(format!("system_bar_visible={:?}", insets.is_system_bar_visible()));
    }

    CONTROLS.with(|controls| {
        if let Some(c) = controls.borrow().as_ref() {
            // The drop target, in the coordinates of the view.
            if let Some(point) =
                c.drop_target.translate_point(ferroui_base::Point::new(80.0, 18.0), &top_level)
            {
                line.push(format!("drop_target={},{}", point.x.round(), point.y.round()));
            }
        }
    });

    CONTROLS.with(|controls| {
        if let Some(c) = controls.borrow().as_ref() {
            // The native control host: whether it has a native control, and its bounds in the
            // coordinates of the view while it is in the view.
            line.push(format!("native_handle={}", c.native_host.native_control_handle().is_some()));
            let size = c.native_host.bounds();
            if let Some(origin) = c.native_host.translate_point(ferroui_base::Point::new(0.0, 0.0), &top_level) {
                line.push(format!("native_host={},{},{},{}", origin.x, origin.y, size.width, size.height));
            }
        }
    });

    SERVICES.with(|services| {
        let services = services.borrow();
        line.push(format!("dropped={:?}", services.dropped));
        line.push(format!("drag_overs={}", services.drag_overs));
        line.push(format!("clipboard={:?}", services.clipboard));
        line.push(format!("screen_details={:?}", services.screen_details));
        line.push(format!("launched={:?}", services.launched));
        line.push(format!("back_requests={}", services.back_requests));
        line.push(format!("safe_area_changes={}", services.safe_area_changes));
    });

    line.join(";")
}

/// Reads the text on the clipboard through the clipboard of the top-level;
/// the answer is reported by [`themed_view_services`] (`clipboard=`).
#[wasm_bindgen(js_name = themedViewReadClipboard)]
pub fn themed_view_read_clipboard() {
    let Some(clipboard) = top_level().and_then(|top_level| top_level.clipboard()) else { return };
    with_services(|services| services.clipboard = None);
    drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
        let text = match clipboard.try_get_text_async().await {
            Ok(text) => format!("ok:{}", text.unwrap_or_default()),
            Err(error) => format!("error:{:?}", error.kind()),
        };
        with_services(|services| services.clipboard = Some(text));
    }));
}

/// Writes text to the clipboard through the clipboard of the top-level.
#[wasm_bindgen(js_name = themedViewWriteClipboard)]
pub fn themed_view_write_clipboard(text: &str) {
    let Some(clipboard) = top_level().and_then(|top_level| top_level.clipboard()) else { return };
    let text = text.to_string();
    with_services(|services| services.clipboard = None);
    drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
        let result = match clipboard.set_text_async(Some(&text)).await {
            Ok(()) => "written".to_string(),
            Err(error) => format!("error:{:?}", error.kind()),
        };
        with_services(|services| services.clipboard = Some(result));
    }));
}

/// Asks for the details of all screens; the answer is reported by
/// [`themed_view_services`] (`screen_details=`).
#[wasm_bindgen(js_name = themedViewRequestScreenDetails)]
pub fn themed_view_request_screen_details() {
    let Some(screens) = top_level().and_then(|top_level| top_level.screens()) else { return };
    drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
        let granted = screens.request_screen_details().await;
        with_services(|services| services.screen_details = Some(granted));
    }));
}

/// Opens a URI with the launcher of the top-level; the answer is reported
/// by [`themed_view_services`] (`launched=`).
#[wasm_bindgen(js_name = themedViewLaunch)]
pub fn themed_view_launch(uri: &str) {
    let Some(launcher) = top_level().map(|top_level| top_level.launcher()) else { return };
    let Ok(uri) = Uri::new(uri, UriKind::RelativeOrAbsolute) else { return };
    drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
        let launched = launcher.launch_uri_async(&uri).await;
        with_services(|services| services.launched = Some(launched));
    }));
}

/// Makes the back navigation of the browser a back request of the view,
/// which the view handles and counts (`back_requests=`).
#[wasm_bindgen(js_name = themedViewInstallBackHandler)]
pub fn themed_view_install_back_handler() {
    let Some(top_level) = top_level() else { return };
    top_level.back_requested(|_, e| {
        with_services(|services| services.back_requests += 1);
        e.set_handled(true);
    });
    navigation_helper::add_back_handler();
}

/// Changes the native control host of the view: `add` puts it into the
/// panel below the title, `remove` takes it out, `size` sets its size to
/// `width` x `height`, `margin` sets its left margin to `width`, and `hide`
/// and `show` set its visibility. What the host then has is reported by
/// [`themed_view_services`] (`native_handle=`, `native_host=`).
#[wasm_bindgen(js_name = themedViewNativeHost)]
pub fn themed_view_native_host(action: &str, width: f64, height: f64) {
    CONTROLS.with(|controls| {
        let controls = controls.borrow();
        let Some(c) = controls.as_ref() else { return };
        let host = &c.native_host;
        match action {
            "add" => c.panel.children().insert(1, host.clone().upcast::<Control>()),
            "remove" => {
                c.panel.children().remove(&host.clone().upcast::<Control>());
            }
            "size" => {
                host.set_width(width);
                host.set_height(height);
            }
            "margin" => host.set_margin(Thickness::new(width, 0.0, 0.0, 0.0)),
            "hide" => host.set_is_visible(false),
            "show" => host.set_is_visible(true),
            _ => {}
        }
    });
}

/// What the panics of [`themed_view_panic`] did.
#[derive(Default)]
struct Panics {
    handler: bool,
    handled: Vec<String>,
    jobs_after: u32,
    invoke: Option<String>,
}

thread_local! {
    static PANICS: RefCell<Panics> = RefCell::new(Panics::default());
}

/// Panics on the dispatcher, for the behaviour tests: the framework must catch
/// and pass them on in the page as it does on the desktop (the `catch_unwind`
/// of the dispatcher's job loop and of `invoke`).
///
/// - `post`: a posted job panics; the `unhandled_exception` handler of the
///   dispatcher sees the panic and marks it handled.
/// - `unhandled`: a posted job panics and the handler leaves it unhandled, so
///   the dispatcher raises it again out of its job loop, to the page.
/// - `invoke`: a callback run with `invoke_local` panics; the panic reaches the
///   caller, which catches it here.
///
/// Every kind but `invoke` then posts a job that counts itself, which shows
/// that the dispatcher still runs jobs. Returns the state line:
/// `handled=<messages, separated by |>;jobs_after=<count>;invoke=<message>`.
#[wasm_bindgen(js_name = themedViewPanic)]
pub fn themed_view_panic(kind: &str) -> String {
    let dispatcher = Dispatcher::ui_thread();
    if !PANICS.with(|panics| std::mem::replace(&mut panics.borrow_mut().handler, true)) {
        let subscription = dispatcher.unhandled_exception(|args| {
            let message = args.exception_message().unwrap_or("?").to_string();
            if message.contains("handled") && !message.contains("unhandled") {
                args.set_handled(true);
            }
            PANICS.with(|panics| panics.borrow_mut().handled.push(message));
        });
        std::mem::forget(subscription);
    }

    match kind {
        "post" => dispatcher.post_local(|| panic!("a posted job panics (handled)"), DispatcherPriority::DEFAULT),
        "unhandled" => dispatcher.post_local(|| panic!("a posted job panics (unhandled)"), DispatcherPriority::DEFAULT),
        "invoke" => {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                dispatcher.invoke_local(|| -> u32 { panic!("an invoked callback panics") })
            }));
            let message = match result {
                Ok(_) => "none".to_string(),
                Err(payload) => payload
                    .downcast_ref::<&str>()
                    .map(|message| message.to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "?".to_string()),
            };
            PANICS.with(|panics| panics.borrow_mut().invoke = Some(message));
        }
        _ => {}
    }
    if kind != "invoke" && !kind.is_empty() {
        dispatcher.post_local(|| PANICS.with(|panics| panics.borrow_mut().jobs_after += 1), DispatcherPriority::DEFAULT);
    }

    PANICS.with(|panics| {
        let panics = panics.borrow();
        format!(
            "handled={};jobs_after={};invoke={}",
            panics.handled.join("|"),
            panics.jobs_after,
            panics.invoke.as_deref().unwrap_or("")
        )
    })
}

/// Decodes damaged pictures, for the behaviour tests: libpng and libjpeg-turbo
/// in Skia's prebuilt archive leave a decode that fails with `longjmp`, which
/// in the browser goes through `emscripten/emscripten_sjlj.cpp` of
/// `ferroui-skia`.
///
/// A 4 x 4 bitmap is saved as PNG and as JPEG and decoded three times: as
/// saved, damaged (PNG: a byte of the header chunk changed, so its checksum
/// fails; JPEG: an image height of 0 in the frame header) and as saved again.
/// Returns `png=<result>,<result>,<result>;jpeg=...`, each result `ok` or
/// `error`.
#[wasm_bindgen(js_name = themedViewDecodeDamaged)]
pub fn themed_view_decode_damaged() -> String {
    let pixels = [0x80u8; 4 * 4 * 4];
    let bitmap = Bitmap::from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Unpremul,
        &pixels,
        PixelSize::new(4, 4),
        Vector::new(96.0, 96.0),
        16,
    );
    let decode = |bytes: &[u8]| if Bitmap::from_stream(&mut &bytes[..]).is_ok() { "ok" } else { "error" };

    let formats: [(&str, BitmapEncoderOptions, fn(&mut [u8])); 2] = [
        ("png", PngBitmapEncoderOptions::DEFAULT.into(), |png| png[16] ^= 0x01),
        ("jpeg", JpegBitmapEncoderOptions::DEFAULT.into(), |jpeg| {
            // The frame header: FF C0, length, precision, height, width.
            if let Some(at) = jpeg.windows(2).position(|marker| marker == [0xFF, 0xC0]) {
                jpeg[at + 5] = 0;
                jpeg[at + 6] = 0;
            }
        }),
    ];
    let mut results = Vec::new();
    for (name, options, damage) in formats {
        let mut saved = Vec::new();
        if bitmap.save(&mut saved, &options).is_err() {
            results.push(format!("{name}=not saved"));
            continue;
        }
        let mut damaged = saved.clone();
        damage(&mut damaged);
        results.push(format!("{name}={},{},{}", decode(&saved), decode(&damaged), decode(&saved)));
    }
    results.join(";")
}

/// Where the frames of the view are rendered, as a line of `name=value`
/// pairs, for the behaviour tests:
///
/// - `frames`: the frames drawn to the canvas so far;
/// - `frame_thread`: the thread that drew the last one, and `page_thread`:
///   the thread of the page, which makes this call (both 0 in a module
///   built without threads); `other_thread`: whether they differ;
/// - `render_thread`: the render thread of the platform, 0 without one, and
///   `on_render_thread`: whether the render loop of the page ticks there;
/// - `kind` (`webgl`, `software`, `none` before the first frame), `gl` (the
///   major version of OpenGL ES: 2 for WebGL 1, 3 for WebGL 2, 0 in
///   software) and `size` (device pixels) of the last frame;
/// - `ticks`: the ticks of the frame loop of the render thread, and
///   `proxied`: the calls those ticks made the main thread of the page
///   serve (`unknown` when they are not counted), with `last_proxied`, the
///   index of the function of the last one in the script of the module (-1
///   for none);
/// - `released`: the canvases of closed views the thread that renders has
///   released, and `panics`: the panics of the render thread;
/// - `renderer`: the render backend the application was started with
///   (`Skia` or `Vello`).
#[wasm_bindgen(js_name = themedViewRendering)]
pub fn themed_view_rendering() -> String {
    let statistics = RenderStatistics::current();
    let page_thread = thread_proxy::current_thread();
    let kind = match statistics.frame_kind {
        RENDER_TARGET_KIND_WEB_GL => "webgl",
        RENDER_TARGET_KIND_SOFTWARE => "software",
        _ => "none",
    };
    format!(
        "frames={};frame_thread={};page_thread={};other_thread={};render_thread={};on_render_thread={};kind={};gl={};size={}x{};ticks={};proxied={};last_proxied={};released={};panics={};renderer={}",
        statistics.frames,
        statistics.frame_thread,
        page_thread,
        statistics.frames > 0 && statistics.frame_thread != page_thread,
        RenderWorker::thread_id(),
        BrowserSharedRenderLoop::renders_on_render_thread(),
        kind,
        statistics.frame_gl_major_version,
        statistics.frame_width,
        statistics.frame_height,
        statistics.ticks,
        statistics.tick_proxied_calls.map_or_else(|| "unknown".to_string(), |calls| calls.to_string()),
        statistics.last_proxied_function,
        statistics.canvases_released,
        statistics.render_thread_panics,
        renderer_name(),
    )
}

/// The render backend the application was started with: `Skia` or `Vello`.
fn renderer_name() -> &'static str {
    FerroLocator::current().get_service::<BrowserPlatformOptions>().map_or(BrowserRenderer::Skia, |options| options.renderer).name()
}

/// Opens (`open`) a second view in the element `second` of the page, which
/// the caller has added, or closes it (`close`); returns `opened`, `closed`
/// or what was wrong. The view shows one colour. For the behaviour tests:
/// a second canvas next to the first one, with a render target and a
/// graphics context of its own on the thread that renders, and the disposal
/// of a view while another goes on drawing (`released=` of
/// [`themed_view_rendering`] counts the canvases the thread that renders
/// has released).
#[wasm_bindgen(js_name = themedViewSecondView)]
pub fn themed_view_second_view(action: &str) -> String {
    match action {
        "open" => {
            if SECOND_VIEW.with(|view| view.borrow().is_some()) {
                return "already open".to_string();
            }
            let view = FerroView::new("second");
            let content = Border::new();
            content.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(0x1f, 0x9d, 0x55)))));
            view.set_content(Some(content.upcast()));
            SECOND_VIEW.with(|slot| *slot.borrow_mut() = Some(view));
            "opened".to_string()
        }
        "close" => match SECOND_VIEW.with(|slot| slot.borrow_mut().take()) {
            Some(view) => {
                view.dispose();
                "closed".to_string()
            }
            None => "not open".to_string(),
        },
        other => format!("unknown action {other}"),
    }
}

/// The value of `name` in a query string (`?a=1&b=2`), ignoring the case of
/// the name.
fn query_value(query: &str, name: &str) -> Option<String> {
    query
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.to_string())
}

fn parse_args(query: &str) -> BrowserPlatformOptions {
    let mut options = BrowserPlatformOptions::default();
    if let Some(rendering_mode_pairs) = query_value(query, "RenderingMode") {
        let modes: Vec<BrowserRenderingMode> = rendering_mode_pairs
            .split(|c| c == ';' || c == ',')
            .filter(|entry| !entry.is_empty())
            .filter_map(BrowserRenderingMode::parse)
            .collect();
        if !modes.is_empty() {
            options.rendering_mode = modes;
        }
    }
    // The Vello backend, in a module that has it. A module without it keeps
    // Skia and says so: the page asked for something that is not there.
    if let Some(renderer) = query_value(query, "Renderer").as_deref().and_then(BrowserRenderer::parse) {
        if renderer.is_available() {
            options.renderer = renderer;
        } else {
            eprintln!("themed_view: ?Renderer={} is not in this module (built without the feature `vello`); Skia draws", renderer.name());
        }
    }
    // A module built with threads renders on a render thread unless the page
    // asks for one thread.
    if query_value(query, "RenderThread").is_some_and(|value| value.eq_ignore_ascii_case("false")) {
        options.render_thread = false;
    }
    options
}

/// The entry point the host page calls once the module is created and
/// registered with the script side. `query` is the query string of the page.
#[wasm_bindgen(js_name = runMain)]
pub fn run_main(query: &str) {
    let options = parse_args(query);
    THEME_VARIANT.with(|variant| *variant.borrow_mut() = query_value(query, "ThemeVariant"));

    AppBuilder::configure::<App>()
        .with_inter_font()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".to_string()),
            ..Default::default()
        }))
        .start_browser_app("out", Some(options));

    // Counts the changes of the safe area the insets manager reports (`safe_area_changes=`).
    if let Some(insets) = top_level().and_then(|top_level| top_level.insets_manager()) {
        let subscription = insets.safe_area_changed(Rc::new(|_| with_services(|services| services.safe_area_changes += 1)));
        std::mem::forget(subscription);
    }
}

/// The example only does something in a web page; elsewhere it builds so
/// that the workspace checks cover it.
#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("themed_view runs in a browser: build it with scripts/build-browser.sh themed_view");
}
