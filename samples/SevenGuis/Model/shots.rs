//! Running a task in a window of its own ([`run`]), and its picture: with
//! `FERROUI_SEVEN_GUIS_SHOTS=<directory>`, the window renders itself into
//! `<directory>/<name>.png` once it is shown, and closes. The window renders
//! into a bitmap of its own size and scaling: the picture is what the window
//! shows, without the desktop around it.

use ferroui_base::media::imaging::{IBitmap, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{PixelSize, Ref, Vector, Visual};
use ferroui_controls::Window;
use ferroui_view::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Runs the element of a task as the content of the main window, titled
/// `title`, of `width` by `height`, in the WinUI 3 theme. `name` names its
/// picture.
pub fn run(name: &'static str, title: &'static str, width: f64, height: f64, view: impl FnOnce() -> Element + 'static) {
    launch_themed(
        || ferroui_themes_winui::WinUITheme::new().as_style(),
        move || window().title(title).width(width).height(height).child(view()).into(),
        move |window| capture_if_asked(window, name),
    );
}

/// Takes the picture of `window` when `FERROUI_SEVEN_GUIS_SHOTS` names a
/// directory, then closes it; does nothing otherwise.
pub fn capture_if_asked(window: &Ref<Window>, name: &'static str) {
    let Some(directory) = std::env::var_os("FERROUI_SEVEN_GUIS_SHOTS").map(PathBuf::from) else { return };
    let _ = std::fs::create_dir_all(&directory);
    let window = window.clone();
    let _timer = DispatcherTimer::run_once(
        move || {
            capture(&window, &directory.join(format!("{name}.png")));
            window.close();
        },
        // `FERROUI_SEVEN_GUIS_SHOTS_DELAY_MS` waits longer (a picture after
        // something changed while the window is shown).
        Duration::from_millis(
            std::env::var("FERROUI_SEVEN_GUIS_SHOTS_DELAY_MS").ok().and_then(|ms| ms.parse().ok()).unwrap_or(1500),
        ),
        DispatcherPriority::NORMAL,
    );
}

fn capture(window: &Ref<Window>, file: &Path) {
    let scaling = window.render_scaling();
    let size = window.client_size();
    let pixels = PixelSize::new((size.width * scaling).round() as i32, (size.height * scaling).round() as i32);
    let bitmap = RenderTargetBitmap::with_dpi(pixels, Vector::new(96.0 * scaling, 96.0 * scaling));
    let visual: Ref<Visual> = window.clone().upcast();
    bitmap.render(&visual);
    let mut out = Vec::new();
    match IBitmap::save(&*bitmap, &mut out, &PngBitmapEncoderOptions::default().into()).and_then(|()| std::fs::write(file, out)) {
        Ok(()) => println!("wrote {}", file.display()),
        Err(error) => eprintln!("could not write {}: {error}", file.display()),
    }
}
