//! The application in a window ([`run`]), and its picture: with
//! `FERROUI_TODOMVC_SHOTS=<directory>`, the window starts with a few todos,
//! renders itself into `<directory>/todo-mvc.png` once it is shown, and
//! closes. The window renders into a bitmap of its own size and scaling: the
//! picture is what the window shows, without the desktop around it.

use crate::model::Todos;
use crate::view::app;
use ferroui_base::media::imaging::{IBitmap, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{PixelSize, Ref, Vector, Visual};
use ferroui_controls::Window;
use ferroui_view::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The variable that names the directory of the picture.
const SHOTS: &str = "FERROUI_TODOMVC_SHOTS";

/// Runs TodoMVC in the main window, in the WinUI 3 theme.
pub fn run() {
    let shots = std::env::var_os(SHOTS).is_some();
    launch_themed(
        || ferroui_themes_winui::WinUITheme::new().as_style(),
        move || {
            // The picture shows a list with todos of both kinds.
            let todos = shots.then(|| {
                signal(Todos::from_titles(&[
                    ("Port TodoMVC to ferroui-view", true),
                    ("Write its scenario tests", false),
                    ("Take its picture", false),
                ]))
            });
            window().title("TodoMVC").width(640.0).height(600.0).child(app().todos(todos)).into()
        },
        |window| capture_if_asked(window, "todo-mvc"),
    );
}

/// Takes the picture of `window` when `FERROUI_TODOMVC_SHOTS` names a
/// directory, then closes it; does nothing otherwise.
pub fn capture_if_asked(window: &Ref<Window>, name: &'static str) {
    let Some(directory) = std::env::var_os(SHOTS).map(PathBuf::from) else { return };
    let _ = std::fs::create_dir_all(&directory);
    let window = window.clone();
    let _timer = DispatcherTimer::run_once(
        move || {
            capture(&window, &directory.join(format!("{name}.png")));
            window.close();
        },
        // `FERROUI_TODOMVC_SHOTS_DELAY_MS` waits longer.
        Duration::from_millis(std::env::var("FERROUI_TODOMVC_SHOTS_DELAY_MS").ok().and_then(|ms| ms.parse().ok()).unwrap_or(1500)),
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
