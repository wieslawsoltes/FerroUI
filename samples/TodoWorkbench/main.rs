//! A small native Todo app for the FerroUI VS Code workflow.

#[cfg(feature = "app-preview")]
mod app_preview;
mod view;

use ferroui_view::prelude::*;
use todo_mvc::model::Todos;

fn main() {
    #[cfg(feature = "app-preview")]
    {
        app_preview::start_if_requested();
        let mut args = std::env::args_os().skip(1);
        if args.any(|arg| arg == "--xaml-live") {
            let path = std::path::PathBuf::from(
                args.next()
                    .expect("--xaml-live requires a XAML window path"),
            );
            ferroui_view::launch_themed(
                || ferroui_themes_winui::WinUITheme::new().as_style(),
                move || match app_preview::load_window(&path) {
                    Ok(window) => window.into_element(),
                    Err(error) => {
                        eprintln!("Could not open {}: {error}", path.display());
                        std::process::exit(1);
                    }
                },
                window_started,
            );
            return;
        }
    }
    let demo = std::env::args().any(|arg| arg == "--demo");
    let initial = if demo {
        Todos::from_titles(&[
            ("Set up the FerroUI workspace", true),
            ("Try the designer Toolbox", false),
            ("Inspect the live Todo app", false),
        ])
    } else {
        Todos::new()
    };
    ferroui_view::launch_themed(
        || ferroui_themes_winui::WinUITheme::new().as_style(),
        move || {
            window()
                .title("FerroUI Todo")
                .width(660.0)
                .height(610.0)
                .child(view::todo_page().initial(initial))
                .into_element()
        },
        window_started,
    );
}

fn window_started(window: &ferroui_base::Ref<Window>) {
    #[cfg(feature = "mcp")]
    start_mcp(window);
    #[cfg(not(feature = "mcp"))]
    let _ = window;
}

#[cfg(feature = "mcp")]
fn start_mcp(window: &ferroui_base::Ref<Window>) {
    if !std::env::args().any(|arg| arg == "--mcp") {
        return;
    }
    let root = window.clone().upcast();
    ferroui_inspect::Inspector::shared()
        .borrow_mut()
        .register_root(&root);
    let allow_input = std::env::args().any(|arg| arg == "--mcp-input");
    match ferroui_mcp::serve(ferroui_mcp::Options {
        permissions: ferroui_mcp::Permissions {
            allow_input,
            ..Default::default()
        },
        ..Default::default()
    }) {
        Ok(server) => {
            let server = std::cell::RefCell::new(Some(server));
            window
                .closed(move || {
                    server.borrow_mut().take();
                })
                .detach();
        }
        Err(error) => eprintln!("Could not start Todo MCP: {error}"),
    }
}
