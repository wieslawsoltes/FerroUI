use crate::services::ApplicationCatalog;
use crate::session::Session;
use crate::shell::appearance::{Appearance, ThemeController};
use crate::shell::{self, Scene};
use ferroui_controls::{AppBuilder, Application, Control, Window};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_themes_material::MaterialAppBuilderExtension;
use ferroui_view::Owner;
use ferroui_x11::{FerroX11PlatformExtensions, X11PlatformOptions, X11RenderingMode};
use std::process::ExitCode;
use std::rc::Rc;

const HELP: &str = "Usage: ferro-desktop [--session] [--software] [--theme=windows|macos|material] [--scene=desktop|applications|workspaces|notifications|settings|calendar]

By default the shell opens a window on DISPLAY using Skia Graphite over Vulkan.
--session   Own the primary desktop surface in a dedicated X11 session.
--software  Use Skia's CPU rasterizer explicitly.
--theme     Start with this appearance (otherwise use the saved preference).
--scene     Select the initial shell view.";

struct Options {
    session: bool,
    software: bool,
    scene: Scene,
    theme: Option<Appearance>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            session: false,
            software: false,
            scene: Scene::Desktop,
            theme: None,
        };
        for arg in args {
            if let Some(id) = arg.strip_prefix("--theme=") {
                options.theme =
                    Some(Appearance::parse(id).ok_or_else(|| format!("Unknown theme '{id}'"))?);
            } else {
                match arg.as_str() {
                    "--session" => options.session = true,
                    "--software" => options.software = true,
                    "--scene=notifications" => options.scene = Scene::Notifications,
                    "--scene=settings" => options.scene = Scene::QuickSettings,
                    "--scene=calendar" => options.scene = Scene::Calendar,
                    "--scene=desktop" => options.scene = Scene::Desktop,
                    "--scene=applications" => options.scene = Scene::Applications,
                    "--scene=workspaces" => options.scene = Scene::Workspaces,
                    _ => return Err(format!("Unknown argument '{arg}'")),
                }
            }
        }
        Ok(options)
    }
}

pub fn run() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let options = match Options::parse(&args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}\n{HELP}");
            return ExitCode::from(2);
        }
    };
    if !options.software && !ferroui_skia::gpu::GRAPHITE_VULKAN {
        eprintln!("This Skia build lacks Vulkan Graphite. Install the Graphite build or start ferro-desktop --software.");
        return ExitCode::FAILURE;
    }
    if std::env::var_os("DISPLAY").is_none_or(|display| display.is_empty()) {
        eprintln!("Ferro Desktop needs an X11 display. Start an X11 session or set DISPLAY.");
        return ExitCode::FAILURE;
    }
    let mut platform = X11PlatformOptions::new();
    platform.wm_class = Some("FerroDesktop".into());
    platform.rendering_mode = vec![if options.software {
        X11RenderingMode::Software
    } else {
        X11RenderingMode::Vulkan
    }];
    let mut vulkan = ferroui_vulkan::VulkanOptions::default();
    vulkan.vulkan_instance_creation_options.application_name = Some("Ferro Desktop".into());
    // Graphite probes the two-plane 4:4:4 formats and chooses driver-specific workarounds.
    vulkan
        .vulkan_device_creation_options
        .device_extensions
        .extend([
            "VK_KHR_driver_properties".into(),
            "VK_EXT_ycbcr_2plane_444_formats".into(),
        ]);
    let theme = options.theme.unwrap_or_else(Appearance::preferred);
    // Let GIO resolve the user's installed entries while X11, Skia and the
    // initial theme load. No native application handles leave the worker.
    let applications = std::thread::spawn(ApplicationCatalog::discover);
    AppBuilder::configure::<Application>()
        .with(Rc::new(platform))
        .with(Rc::new(vulkan))
        .use_x11()
        .use_harfbuzz()
        .use_skia()
        .with_material_fonts()
        .start(
            move |app, _| {
                // System Task workers post their completions to this UI dispatcher.
                // Without the poster, GIO launches, pin edits and settings requests finish
                // on the worker but their continuations remain queued until shutdown.
                let previous_context = ferroui_system::threading::current_synchronization_context();
                ferroui_system::threading::set_synchronization_context(Some(
                    ferroui_base::threading::FerroSynchronizationContext::with_priority(
                        ferroui_base::threading::DispatcherPriority::NORMAL,
                    ),
                ));
                let themes = ThemeController::install(app, theme);
                let session = Session::connect();
                let catalog = Rc::new(applications.join().expect("application discovery worker"));
                let window = Window::new();
                window.set_title(Some("Ferro Desktop".into()));
                window.set_width(1280.0);
                window.set_height(800.0);
                if options.session {
                    session
                        .as_ref()
                        .expect("FerroUI X11 session initialized")
                        .configure_desktop(&window);
                }
                let owner = Owner::new_root();
                let content = owner.with(|| {
                    shell::view(options.scene, catalog, session, themes, options.session).mount()
                });
                window.set_content(Some(Control::boxed(content)));
                eprintln!(
                    "Ferro Desktop / {} / {}",
                    theme.name(),
                    if options.software {
                        "Skia CPU raster"
                    } else {
                        "Skia Graphite Vulkan"
                    }
                );
                app.run_window(window);
                owner.dispose();
                ferroui_system::threading::set_synchronization_context(previous_context);
            },
            &args,
        );
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_defaults_to_graphite_and_rejects_unknown_options() {
        assert!(!Options::parse(&[]).unwrap().software);
        let opts = Options::parse(&[
            "--software".into(),
            "--session".into(),
            "--theme=macos".into(),
            "--scene=applications".into(),
        ])
        .unwrap();
        assert!(opts.software && opts.session);
        assert_eq!(opts.scene, Scene::Applications);
        assert_eq!(opts.theme, Some(Appearance::MacOS));
        assert!(Options::parse(&["--theme=typo".into()]).is_err());
        assert!(Options::parse(&["--scene=typo".into()]).is_err());
    }
}
