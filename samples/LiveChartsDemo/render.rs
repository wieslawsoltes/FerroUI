//! Renders the compiled native view through Vello CPU, including native text.

use ferroui_base::media::imaging::{IBitmap, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Ref, Vector, Visual};
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use ferroui_vello::{VelloApplicationExtensions, VelloOptions, VelloRenderingMode};
use livecharts_demo::{app_builder, DemoView, VIEW_SIZE};
use std::path::PathBuf;
use std::rc::Rc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = PathBuf::from("target/livecharts-demo.png");
    let mut mutated = false;
    for arg in std::env::args().skip(1) {
        if arg == "--mutated" {
            mutated = true;
        } else {
            file = PathBuf::from(arg);
        }
    }
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
    let view = DemoView::new().map_err(|error| error.message().to_owned())?;
    for index in 0..6 {
        view.chart(index).set_animations_speed(0.);
    }
    if mutated {
        view.advance();
    }
    let window = view.create_window();
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    view.layout(VIEW_SIZE);
    let counts = view.data_counts()?;
    if counts.len() != 6 || counts.iter().any(|(_, count)| *count == 0) {
        return Err(format!("every chart must contain data after layout: {counts:?}").into());
    }
    let bitmap = RenderTargetBitmap::with_dpi(
        PixelSize::new(VIEW_SIZE.width as i32, VIEW_SIZE.height as i32),
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
    if let Some(parent) = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&file, bytes)?;
    window.close();
    println!(
        "wrote {} (Vello CPU, compiled XAML, data hits: {counts:?})",
        file.display()
    );
    Ok(())
}
