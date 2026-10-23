//! Native launchers using FerroUI platform providers and the Vello render backend.
//!
//! The hybrid renderer draws through Metal on macOS and the platform OpenGL
//! context on Linux/X11 and Windows. CPU framebuffers remain the fallback;
//! `--software` explicitly requests that path.

use ferroui_controls::AppBuilder;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_vello::{VelloApplicationExtensions, VelloOptions, VelloRenderingMode};
use std::rc::Rc;

/// Configure services without creating a window or connecting to a display server.
pub fn configure(builder: AppBuilder, software: bool) -> AppBuilder {
    let renderer = if software {
        VelloOptions::with_rendering_mode(VelloRenderingMode::Cpu)
    } else {
        VelloOptions {
            rendering_modes: [
                Some(VelloRenderingMode::Hybrid),
                Some(VelloRenderingMode::Cpu),
                None,
            ],
            ..VelloOptions::default()
        }
    };
    let builder = builder.with(Rc::new(renderer)).use_harfbuzz().use_vello();

    #[cfg(target_os = "macos")]
    {
        use ferroui_native::{
            FerroNativePlatformExtensions, FerroNativePlatformOptions, FerroNativeRenderingMode,
        };
        let modes = if software {
            vec![FerroNativeRenderingMode::Software]
        } else {
            vec![
                FerroNativeRenderingMode::Metal,
                FerroNativeRenderingMode::Software,
            ]
        };
        builder
            .with(Rc::new(FerroNativePlatformOptions {
                rendering_mode: modes,
                ..FerroNativePlatformOptions::default()
            }))
            .use_ferro_native()
    }

    #[cfg(target_os = "linux")]
    {
        use ferroui_x11::{FerroX11PlatformExtensions, X11PlatformOptions, X11RenderingMode};
        let modes = if software {
            vec![X11RenderingMode::Software]
        } else {
            vec![X11RenderingMode::Glx, X11RenderingMode::Software]
        };
        builder
            .with(Rc::new(X11PlatformOptions {
                rendering_mode: modes,
                wm_class: Some("FerroDesign".to_owned()),
                use_retained_framebuffer: Some(true),
                use_x_shm_framebuffer: Some(true),
                ..X11PlatformOptions::default()
            }))
            .use_x11()
    }

    #[cfg(windows)]
    {
        use ferroui_win32::{Win32ApplicationExtensions, Win32PlatformOptions, Win32RenderingMode};
        let modes = if software {
            vec![Win32RenderingMode::Software]
        } else {
            vec![Win32RenderingMode::Wgl, Win32RenderingMode::Software]
        };
        builder
            .with(Rc::new(Win32PlatformOptions {
                rendering_mode: modes,
                ..Win32PlatformOptions::default()
            }))
            .use_win32()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        let _ = (builder, software);
        panic!("Native FerroDesign supports Linux/X11, macOS and Windows");
    }
}

/// Run the desktop lifetime, or a requested diagnostic operation.
pub fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(index) = args.iter().position(|arg| arg == "--render-smoke") {
        let Some(path) = args.get(index + 1) else {
            eprintln!("Usage: ferrodesign-app --render-smoke <output.png>");
            return 2;
        };
        return match render_smoke(std::path::Path::new(path)) {
            Ok(()) => {
                println!("Rendered FerroDesign with Vello CPU: {path}");
                0
            }
            Err(error) => {
                eprintln!("FerroDesign render smoke failed: {error}");
                1
            }
        };
    }

    let builder = configure(
        crate::app::build_app(),
        args.iter().any(|arg| arg == "--software"),
    );
    if args.iter().any(|arg| arg == "--renderer-info") {
        println!(
            "Configured renderer: {}; text shaping: {}; windowing: {}",
            builder.rendering_subsystem_name().unwrap_or_default(),
            builder.text_shaping_subsystem_name().unwrap_or_default(),
            builder.windowing_subsystem_name().unwrap_or_default(),
        );
        return 0;
    }
    builder.start_with_classic_desktop_lifetime(&args)
}

/// Draw the compiled-XAML application into a real Vello framebuffer and save a PNG.
pub fn render_smoke(path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessWindowExtensions,
    };

    let builder = crate::app::build_app()
        .use_harfbuzz()
        .with(Rc::new(VelloOptions::with_rendering_mode(
            VelloRenderingMode::Cpu,
        )))
        .use_vello()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            ..FerroHeadlessPlatformOptions::default()
        })
        .setup_with_classic_desktop_lifetime(&[]);
    let app = builder
        .instance()
        .ok_or_else(|| std::io::Error::other("Application was not initialized"))?;
    let lifetime = app
        .application_lifetime()
        .ok_or_else(|| std::io::Error::other("Application has no lifetime"))?;
    let desktop = lifetime
        .as_classic_desktop_style_application_lifetime()
        .ok_or_else(|| std::io::Error::other("Application has no desktop lifetime"))?;
    let window = desktop
        .main_window()
        .ok_or_else(|| std::io::Error::other("Application has no main window"))?;
    window.set_width(1600.0);
    window.set_height(1000.0);
    window.show();
    let frame = window
        .capture_rendered_frame()
        .ok_or_else(|| std::io::Error::other("Vello produced no frame"))?;
    let mut file = std::fs::File::create(path)?;
    frame.save(
        &mut file,
        &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::default()),
    )?;
    window.close();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::Application;

    #[test]
    fn native_services_select_vello_without_opening_a_display() {
        let builder = configure(AppBuilder::configure::<Application>(), true);
        assert_eq!(builder.rendering_subsystem_name().as_deref(), Some("Vello"));
        assert_eq!(
            builder.text_shaping_subsystem_name().as_deref(),
            Some("HarfBuzz")
        );
        #[cfg(target_os = "linux")]
        assert_eq!(builder.windowing_subsystem_name().as_deref(), Some("X11"));
        #[cfg(target_os = "macos")]
        assert_eq!(
            builder.windowing_subsystem_name().as_deref(),
            Some("FerroNative")
        );
        #[cfg(windows)]
        assert_eq!(builder.windowing_subsystem_name().as_deref(), Some("Win32"));
        assert!(builder.instance().is_none());
    }
}
