//! Launches the `PhysicsSandbox` desktop window:
//! - macOS native Metal: `cargo run --profile fast-release -p physics-sandbox --no-default-features --features quartz-metal`
//! - Default Skia:       `cargo run --profile fast-release -p physics-sandbox`
//! - Smoke test:         `FERROUI_SMOKE_EXIT_MS=1500 cargo run -p physics-sandbox --no-default-features --features quartz-metal`

use ferroui_controls::{AppBuilder, Application, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_view::prelude::*;

ferroui_markup_xaml::include_compiled_xaml!();
use compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as THEME_PARTS;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let builder = AppBuilder::configure::<Application>();

    #[cfg(all(target_os = "macos", feature = "quartz"))]
    let builder = builder.with(std::rc::Rc::new(
        ferroui_native::FerroNativePlatformOptions {
            rendering_mode: vec![if cfg!(feature = "quartz-metal") {
                ferroui_native::FerroNativeRenderingMode::Metal
            } else {
                ferroui_native::FerroNativeRenderingMode::Software
            }],
            ..Default::default()
        },
    ));

    builder.use_platform_detect().start(
        move |app, _| {
            app.styles()
                .add(ferroui_themes_winui::WinUITheme::with_parts(None, THEME_PARTS).as_style());
            let owner = Owner::new_root();
            let win = owner.with(|| {
                let control = window()
                    .title("FerroUI Physics Engine — Hardware-Accelerated Sandbox")
                    .width(1280.0)
                    .height(800.0)
                    .child(physics_sandbox::physics_sandbox())
                    .into_element()
                    .mount();
                control.cast::<Window>().expect("root element is a Window")
            });
            win.closed(move || owner.dispose()).detach();

            if let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
            {
                let close_win = win.clone();
                let _timer = ferroui_base::threading::DispatcherTimer::run_once(
                    move || close_win.close(),
                    std::time::Duration::from_millis(ms),
                    ferroui_base::threading::DispatcherPriority::NORMAL,
                )
                .detach();
            }

            app.run_window(win);
        },
        &args,
    );
}
