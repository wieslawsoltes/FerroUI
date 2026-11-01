use crate::services::ApplicationCatalog;
use crate::session::{commands, wayland_commands, Session};
use crate::shell::appearance::{Appearance, ThemeController};
use crate::shell::{self, Scene};
use ferroui_controls::{AppBuilder, Application, Control, Window};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_themes_material::MaterialAppBuilderExtension;
use ferroui_view::Owner;
use ferroui_wayland::{
    FerroWaylandPlatformExtensions, WaylandPlatformOptions, WaylandRenderingMode,
};
use ferroui_x11::{FerroX11PlatformExtensions, X11PlatformOptions, X11RenderingMode};
use std::process::ExitCode;
use std::rc::Rc;

const HELP: &str = "Usage: ferro-desktop [--platform=wayland|x11|auto] [--session] [--software] [--theme=windows|macos|material] [--scene=desktop|applications|workspaces|notifications|settings|calendar]

By default the shell opens a native window on the current Wayland or X11 display using Skia Graphite over Vulkan.
--session   Run desktop services and panels in a dedicated Sway or Openbox session.
--software  Use Skia's CPU rasterizer explicitly.
--theme     Start with this appearance (otherwise use the saved preference).
--scene     Select the initial shell view.
--current-scene  Print the scene displayed by the running login session.
--show=applications|workspaces|settings|notifications   Toggle a surface in the running login session.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Platform {
    X11,
    Wayland,
}

impl Platform {
    fn select(explicit: Option<&str>, wayland: bool, x11: bool) -> Result<Self, String> {
        match explicit.unwrap_or("auto") {
            "wayland" if wayland => Ok(Self::Wayland),
            "x11" if x11 => Ok(Self::X11),
            "auto" if wayland => Ok(Self::Wayland),
            "auto" if x11 => Ok(Self::X11),
            "auto" | "wayland" | "x11" => {
                Err("No display for the selected platform. Set WAYLAND_DISPLAY or DISPLAY.".into())
            }
            name => Err(format!("Unknown platform '{name}'")),
        }
    }
}

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
    let (selectors, args): (Vec<_>, Vec<_>) = args
        .into_iter()
        .partition(|arg| arg.starts_with("--platform="));
    if selectors.len() > 1 {
        eprintln!("Specify --platform once.");
        return ExitCode::from(2);
    }
    let present = |name| std::env::var_os(name).is_some_and(|v| !v.is_empty());
    let platform = match Platform::select(
        selectors
            .first()
            .and_then(|s| s.strip_prefix("--platform=")),
        present("WAYLAND_DISPLAY") || present("WAYLAND_SOCKET"),
        present("DISPLAY"),
    ) {
        Ok(value) => value,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    if let [argument] = args.as_slice() {
        if argument == "--current-scene" {
            let scene = match platform {
                Platform::Wayland => wayland_commands::current_scene(),
                Platform::X11 => commands::current_scene(),
            };
            return match scene {
                Ok(scene) => {
                    println!("{scene}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            };
        }
        if let Some(name) = argument.strip_prefix("--show=") {
            let Some(command) = commands::Command::parse(name) else {
                eprintln!("Unknown desktop command '{name}'");
                return ExitCode::from(2);
            };
            let sent = match platform {
                Platform::Wayland => wayland_commands::send(command),
                Platform::X11 => commands::send(command),
            };
            return match sent {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            };
        }
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
    // Let GIO resolve the user's installed entries while the window system, Skia and the
    // initial theme load. No native application handles leave the worker.
    let applications = std::thread::spawn(ApplicationCatalog::discover);
    let exit_code = std::cell::Cell::new(ExitCode::SUCCESS);
    let result = &exit_code;
    let builder = AppBuilder::configure::<Application>();
    let builder = match platform {
        Platform::X11 => {
            let mut x11 = X11PlatformOptions::new();
            x11.wm_class = Some("FerroDesktop".into());
            x11.rendering_mode = vec![if options.software {
                X11RenderingMode::Software
            } else {
                X11RenderingMode::Vulkan
            }];
            builder.with(Rc::new(x11)).with(Rc::new(vulkan)).use_x11()
        }
        Platform::Wayland => {
            let wayland = WaylandPlatformOptions {
                app_id: Some("org.ferroui.Desktop".into()),
                rendering_mode: if options.software {
                    WaylandRenderingMode::Software
                } else {
                    WaylandRenderingMode::Vulkan(vulkan)
                },
                ..Default::default()
            };
            builder.with(Rc::new(wayland)).use_wayland()
        }
    };
    builder
        .use_harfbuzz()
        .use_skia()
        .with_material_fonts()
        .start(
            move |app, _| {
                let session = match platform {
                    Platform::X11 => Session::connect(),
                    Platform::Wayland => match Session::connect_wayland() {
                        Ok(session) => Some(session),
                        Err(error) if options.session => {
                            eprintln!("Wayland login session: {error}");
                            result.set(ExitCode::FAILURE);
                            return;
                        }
                        Err(_) => None, // Ordinary xdg-shell clients do not need compositor IPC.
                    },
                };
                if options.session {
                    let Some(manager) = session.as_ref() else {
                        eprintln!("The session window manager is unavailable");
                        result.set(ExitCode::FAILURE);
                        return;
                    };
                    if let Err(error) = manager.claim_commands() {
                        eprintln!("{error}");
                        result.set(ExitCode::FAILURE);
                        return;
                    }
                }
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
                let catalog = Rc::new(applications.join().expect("application discovery worker"));
                let owner = Owner::new_root();
                eprintln!(
                    "Ferro Desktop / {} / {} / {platform:?}",
                    theme.name(),
                    if options.software {
                        "Skia CPU raster"
                    } else {
                        "Skia Graphite Vulkan"
                    }
                );
                if platform == Platform::Wayland && options.session {
                    let session = session.expect("managed Wayland session initialized");
                    if let Err(e) = owner
                        .with(|| shell::wayland::run(app, options.scene, catalog, session, themes))
                    {
                        eprintln!("Wayland desktop: {e}");
                        result.set(ExitCode::FAILURE);
                    }
                } else {
                    let window = Window::new();
                    window.set_title(Some("Ferro Desktop".into()));
                    window.set_width(1280.0);
                    window.set_height(800.0);
                    if options.session {
                        session
                            .as_ref()
                            .expect("desktop session initialized")
                            .configure_desktop(&window);
                    }
                    let content = owner.with(|| {
                        shell::view(options.scene, catalog, session, themes, options.session)
                            .mount()
                    });
                    window.set_content(Some(Control::boxed(content)));
                    app.run_window(window);
                }
                owner.dispose();
                ferroui_system::threading::set_synchronization_context(previous_context);
            },
            &args,
        );
    exit_code.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_wayland_wins_when_xwayland_is_also_available() {
        assert_eq!(Platform::select(None, true, true), Ok(Platform::Wayland));
        assert_eq!(Platform::select(Some("x11"), true, true), Ok(Platform::X11));
        assert_eq!(Platform::select(None, false, true), Ok(Platform::X11));
        assert!(Platform::select(Some("wayland"), false, true).is_err());
    }

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
