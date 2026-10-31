//! Renders the ported scenarios of the upstream snapshot suite (`snapshots.rs`) through Vello CPU,
//! one PNG per scenario, named after its upstream reference image.

use ferroui_base::media::imaging::{IBitmap, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Ref, Vector, Visual};
use ferroui_controls::{Control, Window};
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use ferroui_vello::{VelloApplicationExtensions, VelloOptions, VelloRenderingMode};
use livecharts_demo::{app_builder, snapshots};
use std::path::PathBuf;
use std::rc::Rc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/livecharts-snapshots"));
    std::fs::create_dir_all(&directory)?;
    app_builder()
        .with(Rc::new(VelloOptions::with_rendering_mode(
            VelloRenderingMode::Cpu,
        )))
        .use_vello()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            ..FerroHeadlessPlatformOptions::default()
        })
        .setup_without_starting();
    let scenarios = snapshots::scenarios();
    for scenario in &scenarios {
        let chart = snapshots::measure(scenario)
            .map_err(|error| format!("{}: {error:?}", scenario.name))?;
        let window = Window::new();
        window.set_width(scenario.size.width);
        window.set_height(scenario.size.height);
        window.set_background(Some(Rc::new(
            ferroui_base::media::immutable::ImmutableSolidColorBrush::new(
                ferroui_base::media::Color::from_rgb(255, 255, 255),
            ),
        )));
        window.set_content(Some(Control::boxed(chart.clone().upcast::<Control>())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let bitmap = RenderTargetBitmap::with_dpi(
            PixelSize::new(scenario.size.width as i32, scenario.size.height as i32),
            Vector::new(96., 96.),
        );
        let visual: Ref<Visual> = window.clone().upcast();
        bitmap.render(&visual);
        let mut bytes = Vec::new();
        IBitmap::save(
            &*bitmap,
            &mut bytes,
            &PngBitmapEncoderOptions::default().into(),
        )?;
        std::fs::write(directory.join(format!("{}.png", scenario.name)), bytes)?;
        window.close();
    }
    println!(
        "wrote {} scenarios to {} (Vello CPU)",
        scenarios.len(),
        directory.display()
    );
    Ok(())
}
