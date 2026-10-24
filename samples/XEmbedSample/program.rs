//! Port of `Program.cs` of `XEmbedSample`: a window of GTK 3 with controls of GTK above a socket
//! that embeds, through XEmbed, a plug of the framework showing the text box page of the
//! ControlCatalog in a scroll viewer.
//!
//! ```text
//! cargo run -p xembed-sample                     # on Linux, with an X server (DISPLAY) and GTK 3
//! FERROUI_SMOKE_EXIT_MS=6000 FERROUI_SMOKE_SCREENSHOT=/path/xembed.png cargo run -p xembed-sample
//! ```
//!
//! The application of the catalog is set up without a lifetime on the X11 platform with the main
//! loop of GLib (`UseGLibMainLoop`), so that the main loop of GTK runs the dispatcher of the
//! framework too; then GTK is initialized for X11 and its main loop runs until the window is
//! destroyed.
//!
//! Not ported: `HarfbuzzWorkaround.cs`, which the reference calls first. It works around a
//! conflict of the .NET runtime: GtkSharp opens GTK with `RTLD_GLOBAL`, and the HarfBuzz library
//! of SkiaSharp (`libHarfBuzzSharp.so`, a shared library of its own) then resolves some of its
//! own symbols from the HarfBuzz of the system, which crashes; the workaround reopens
//! `libHarfBuzzSharp.so` with `RTLD_DEEPBIND`. The port has no such library: HarfBuzz is built
//! into the executable (`harfbuzz-sys` with its bundled sources), whose references to it are
//! bound when the executable is linked, so the HarfBuzz the system's GTK loads cannot take its
//! place (`GAPS.md`, XE001).
//!
//! Additions of the port, for the smoke runs: with `FERROUI_SMOKE_EXIT_MS=<n>` the program
//! prints `XEMBED SHOWN` with the window of the plug and the size of the content half way, writes
//! a picture of the embedded content (drawn by the framework, not a capture of the screen) to
//! `FERROUI_SMOKE_SCREENSHOT` when it is set, and destroys the window of GTK after `n`
//! milliseconds, which ends the main loop as closing the window does.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("xembed-sample: XEmbed is a protocol of the X server; the sample needs Linux and GTK 3");
}

#[cfg(target_os = "linux")]
mod gtk;
#[cfg(target_os = "linux")]
mod socket_ex;

#[cfg(target_os = "linux")]
fn main() {
    program::main();
}

#[cfg(target_os = "linux")]
mod program {
    use crate::gtk::{self, api, c_string, GdkRgba, Widget, GTK_ORIENTATION_VERTICAL, GTK_STATE_FLAG_NORMAL, GTK_WINDOW_TOPLEVEL};
    use crate::socket_ex::FerroXEmbedGtkSocket;
    use control_catalog::models::CatalogTheme;
    use control_catalog::pages::TextBoxPage;
    use control_catalog::App;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_controls::primitives::ScrollBarVisibility;
    use ferroui_controls::{AppBuilder, Control, ScrollViewer};
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_skia::SkiaApplicationExtensions;
    use ferroui_x11::{FerroX11PlatformExtensions, X11PlatformOptions};
    use std::ffi::c_void;
    use std::rc::Rc;
    use std::time::Duration;

    pub fn main() {
        let mut options = X11PlatformOptions::new();
        options.use_g_lib_main_loop = true;
        options.external_g_lib_main_loop_exception_logger = Some(Rc::new(|e| println!("{}", panic_text(e))));
        AppBuilder::configure::<App>()
            .use_skia()
            .use_harfbuzz()
            .with(Rc::new(options))
            .use_x11()
            .setup_without_starting();
        // The original selects the Fluent theme; the catalog of the port is built with its WinUI
        // theme alone unless the feature `classic-themes` of control-catalog is on
        // (docs/binary-size.md).
        App::set_catalog_themes(CatalogTheme::WinUI);
        let api = api();
        // SAFETY: the initialization of GTK, once, on the main thread, which runs its main
        // loop; the texts are terminated strings that live for the calls.
        unsafe {
            (api.gdk_set_allowed_backends)(c"x11".as_ptr());
            (api.gtk_init)(std::ptr::null_mut(), std::ptr::null_mut());
        }

        // SAFETY (the block): GTK is initialized on this thread; every widget is one of these
        // calls' own results, alive while the window is.
        unsafe {
            let w = (api.gtk_window_new)(GTK_WINDOW_TOPLEVEL);
            (api.gtk_window_set_title)(w, c_string("XEmbed Test Window").as_ptr());
            let socket = FerroXEmbedGtkSocket::new(get_background_color(w));
            let scroll_viewer = ScrollViewer::new();
            scroll_viewer.set_content(Some(Control::boxed(TextBoxPage::new())));
            scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);
            socket.set_content(Some(Control::boxed(scroll_viewer)));
            let vbox = (api.gtk_box_new)(GTK_ORIENTATION_VERTICAL, 5);
            let label = (api.gtk_label_new)(c_string("Those are GTK controls").as_ptr());
            (api.gtk_container_add)(vbox, label);
            (api.gtk_container_add)(vbox, (api.gtk_entry_new)());
            // `new Gtk.Button(new Gtk.Label("Do nothing"))`.
            let button = (api.gtk_button_new)();
            (api.gtk_container_add)(button, (api.gtk_label_new)(c_string("Do nothing").as_ptr()));
            (api.gtk_container_add)(vbox, button);
            (api.gtk_box_pack_end)(vbox, socket.widget(), 1, 1, 0);
            // `HeightRequest` and `WidthRequest`.
            (api.gtk_widget_set_size_request)(socket.widget(), 400, 400);
            (api.gtk_container_add)(w, vbox);
            (api.gtk_widget_realize)(socket.widget());

            gtk::signal_connect(w, c"destroy", on_window_destroyed as *const c_void, socket.widget(), false);
            (api.gtk_widget_show_all)(w);
            smoke_run(w, socket.clone());
            (api.gtk_main)();
        }
    }

    /// The handler of `destroy` of the window: `Gtk.Application.Quit(); socket.Destroy();`.
    extern "C" fn on_window_destroyed(_window: Widget, socket: *mut c_void) {
        // SAFETY: the main loop runs; the socket is a child of the window that is being
        // destroyed, alive until it is destroyed here.
        unsafe {
            (api().gtk_main_quit)();
            (api().gtk_widget_destroy)(socket);
        }
    }

    /// `window.StyleContext.GetBackgroundColor(StateFlags.Normal)` (obsolete in GTK 3, as the
    /// reference says, and fine for a sample).
    fn get_background_color(window: Widget) -> GdkRgba {
        let mut color = GdkRgba::default();
        // SAFETY: the window is alive; the color is a local the call writes.
        unsafe {
            let context = (api().gtk_widget_get_style_context)(window);
            (api().gtk_style_context_get_background_color)(context, GTK_STATE_FLAG_NORMAL, &mut color);
        }
        color
    }

    /// The text of a panic, as `Exception.ToString()` gives that of an exception.
    fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_else(|| "a panic without a message".to_string())
    }

    /// The smoke run asked for with `FERROUI_SMOKE_EXIT_MS` and `FERROUI_SMOKE_SCREENSHOT`.
    fn smoke_run(window: Widget, socket: Rc<FerroXEmbedGtkSocket>) {
        let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|value| value.parse::<u64>().ok()) else {
            return;
        };
        println!("Will destroy the window after {ms} ms");
        let _shown = DispatcherTimer::run_once(
            move || {
                let content = socket.content().and_then(|content| ferroui_base::data::core::ValueTypes::as_object(&*content));
                let control = content.and_then(|content| content.cast::<Control>());
                let size = control.as_ref().map(|control| control.bounds()).unwrap_or_default();
                println!(
                    "XEMBED SHOWN plug window {:?}, content {}x{}",
                    socket.plug_handle(),
                    size.width,
                    size.height
                );
                if let (Some(path), Some(control)) =
                    (std::env::var_os("FERROUI_SMOKE_SCREENSHOT").filter(|path| !path.is_empty()), control)
                {
                    let path = std::path::PathBuf::from(path);
                    match save_control(&control, &path) {
                        Ok(()) => println!("Screenshot: {} (the embedded content, drawn by the framework)", path.display()),
                        Err(error) => println!("Screenshot: {} could not be written: {error}", path.display()),
                    }
                }
            },
            Duration::from_millis(ms / 2),
            DispatcherPriority::NORMAL,
        );
        let window = window as usize;
        let _timer = DispatcherTimer::run_once(
            move || {
                println!("Destroying the window");
                // SAFETY: the window is alive until it is destroyed here.
                unsafe { (api().gtk_widget_destroy)(window as Widget) };
                println!("XEMBED DONE");
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        );
    }

    /// Draws a control into a bitmap of its size through the render target of the framework
    /// and writes it as a PNG file.
    fn save_control(control: &ferroui_base::Ref<Control>, path: &std::path::Path) -> std::io::Result<()> {
        use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions, RenderTargetBitmap};
        use ferroui_base::{PixelSize, Vector, Visual};
        let bounds = control.bounds();
        let size = PixelSize::new(bounds.width.ceil() as i32, bounds.height.ceil() as i32);
        if size.width < 1 || size.height < 1 {
            return Err(std::io::Error::other(format!("the content is empty: {size:?}")));
        }
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let bitmap = RenderTargetBitmap::with_dpi(size, Vector::new(96.0, 96.0));
        bitmap.render(&control.clone().upcast::<Visual>());
        let result = bitmap.save_to_file(&path.to_string_lossy(), &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT));
        bitmap.dispose();
        result
    }
}
