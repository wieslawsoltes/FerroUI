//! The application in a window ([`run`]), and its picture: with
//! `FERROUI_REACTIVE_DEMO_SHOTS=<directory>`, the window searches the in-memory catalogue for
//! `http`, renders itself into `<directory>/reactive-demo-components.png` once the results are
//! shown, and closes. The window renders into a bitmap of its own size and scaling: the picture
//! is what the window shows, without the desktop around it.

use crate::view::{app, Browser};
use ferroui_base::media::imaging::{IBitmap, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{PixelSize, Ref, Vector, Visual};
use ferroui_controls::Window;
use ferroui_view::prelude::*;
use reactive_demo::crate_search::{default_search_service, CatalogueSearchService, ICrateSearchService};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

/// The variable that names the directory of the picture.
const SHOTS: &str = "FERROUI_REACTIVE_DEMO_SHOTS";

/// Runs the crate browser in the main window, in the WinUI 3 theme (`Crate Browser`, 800 by
/// 450, as the window of the reference).
pub fn run() {
    let shots = std::env::var_os(SHOTS).is_some();
    launch_themed(
        || ferroui_themes_winui::WinUITheme::new().as_style(),
        move || {
            // The picture searches the catalogue, whatever the features: it does not depend on
            // the network.
            let service: Rc<dyn ICrateSearchService> =
                if shots { Rc::new(CatalogueSearchService::new()) } else { default_search_service() };
            let browser = Browser::new(service);
            if shots {
                browser.search_text.set(String::from("http"));
            }
            window().title("Crate Browser").width(800.0).height(450.0).child(app().browser(Some(browser))).into()
        },
        |window| capture_if_asked(window, "reactive-demo-components"),
    );
}

/// Takes the picture of `window` when `FERROUI_REACTIVE_DEMO_SHOTS` names a directory, then
/// closes it; does nothing otherwise.
pub fn capture_if_asked(window: &Ref<Window>, name: &'static str) {
    let Some(directory) = std::env::var_os(SHOTS).map(PathBuf::from) else { return };
    let _ = std::fs::create_dir_all(&directory);
    let window = window.clone();
    let _timer = DispatcherTimer::run_once(
        move || {
            capture(&window, &directory.join(format!("{name}.png")));
            window.close();
        },
        // The throttle of the search and the search itself happen first;
        // `FERROUI_REACTIVE_DEMO_SHOTS_DELAY_MS` waits longer.
        Duration::from_millis(
            std::env::var("FERROUI_REACTIVE_DEMO_SHOTS_DELAY_MS").ok().and_then(|ms| ms.parse().ok()).unwrap_or(2000),
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
