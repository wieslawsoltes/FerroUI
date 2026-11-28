//! Reproducible headless captures of the actual WinUI 3 sample, rendered by Skia.
//! `cargo run -p physics-sandbox --no-default-features --features preview --bin physics-studio-preview -- ui dark preview.png 1280 800`

use ferroui_base::media::imaging::{IBitmap, PngBitmapEncoderOptions};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{Dispatcher, DispatcherFrame, DispatcherPriority, DispatcherTimer};
use ferroui_controls::{AppBuilder, Application, Window};
use ferroui_headless::{
    FerroHeadlessPlatform, FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions,
    HeadlessWindowExtensions,
};
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_view::prelude::*;

ferroui_markup_xaml::include_compiled_xaml!();
use compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as THEME_PARTS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let page = args.first().map(String::as_str).unwrap_or("home");
    let dark = args.get(1).is_some_and(|arg| arg == "dark");
    let output = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("physics-studio.png");
    let width: f64 = args
        .get(3)
        .map(|v| v.parse())
        .transpose()?
        .unwrap_or(1280.0);
    let height: f64 = args.get(4).map(|v| v.parse()).transpose()?.unwrap_or(800.0);
    if !(width.is_finite() && height.is_finite() && width >= 320.0 && height >= 320.0) {
        return Err("preview dimensions must be finite and at least 320 DIPs".into());
    }
    // Startup selection is the same path the desktop sample uses.
    std::env::set_var("FERROUI_START_PAGE", page);
    AppBuilder::configure::<Application>()
        .use_skia()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            should_render_on_ui_thread: true,
            ..Default::default()
        })
        .setup_without_starting();
    let app = Application::current().ok_or("application setup failed")?;
    app.set_requested_theme_variant(Some(if dark {
        ThemeVariant::dark()
    } else {
        ThemeVariant::light()
    }));
    app.styles()
        .add(ferroui_themes_winui::WinUITheme::with_parts(None, THEME_PARTS).as_style());
    let owner = Owner::new_root();
    let window = owner.with(|| {
        window()
            .width(width)
            .height(height)
            .child(physics_sandbox::physics_sandbox())
            .into_element()
            .mount()
            .cast::<Window>()
            .expect("root is a window")
    });
    window.show();
    // Pump a bounded real dispatcher frame so rendering and application timers
    // advance together. Waiting for an idle queue can starve on an expensive
    // continuously animated scene, while draining an empty queue skips timers.
    FerroHeadlessPlatform::force_render_timer_tick(1);
    let settle_frame = DispatcherFrame::new();
    let finish = settle_frame.clone();
    let _settle_timer = DispatcherTimer::run_once(
        move || finish.set_continue(false),
        std::time::Duration::from_millis(400),
        DispatcherPriority::NORMAL,
    );
    Dispatcher::ui_thread().push_frame(&settle_frame);
    let frame = window
        .get_last_rendered_frame()
        .ok_or("renderer produced no frame")?;
    frame.save(
        &mut std::fs::File::create(output)?,
        &PngBitmapEncoderOptions::default().into(),
    )?;
    println!(
        "Captured {page} / {} at {width}×{height}: {output}",
        if dark { "dark" } else { "light" }
    );
    window.close();
    owner.dispose();
    Ok(())
}
