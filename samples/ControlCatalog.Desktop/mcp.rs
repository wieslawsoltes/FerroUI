//! Optional inspection server. Feature activation alone does not open a listener.
use ferroui_controls::AppBuilder;
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use ferroui_mcp::{Options, Permissions, ServerHandle, Transport};
use std::{cell::RefCell, rc::Rc};
ferroui_base::ferro_thread_local! { static SERVER:RefCell<Option<ServerHandle>>=const {RefCell::new(None)}; }
pub fn configure(
    builder: AppBuilder,
    args: &[String],
    output: Option<std::fs::File>,
) -> AppBuilder {
    let stdio = args.iter().any(|a| a == "--mcp-stdio");
    let http = args.iter().find_map(|a| a.strip_prefix("--mcp-http="));
    assert!(
        !(stdio && http.is_some()),
        "Choose --mcp-stdio or --mcp-http=<port>"
    );
    if !stdio && http.is_none() {
        return builder;
    }
    let options = Rc::new(RefCell::new(Some(Options {
        transport: if stdio {
            Transport::Stdio
        } else {
            Transport::Http(
                (
                    [127, 0, 0, 1],
                    http.unwrap().parse().expect("MCP port must be 0–65535"),
                )
                    .into(),
            )
        },
        permissions: Permissions {
            allow_input: args.iter().any(|a| a == "--mcp-allow-input"),
            allow_property_writes: args.iter().any(|a| a == "--mcp-allow-writes"),
        },
        stdio_output: output,
        ..Options::default()
    })));
    let builder = if args.iter().any(|a| a == "--mcp-headless") {
        builder.use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            ..Default::default()
        })
    } else {
        builder
    };
    builder.after_setup(move |builder| {
        let handle = ferroui_mcp::serve(options.borrow_mut().take().expect("MCP starts once"))
            .expect("start MCP");
        SERVER.with(|s| *s.borrow_mut() = Some(handle));
        if stdio {
            ferroui_base::threading::DispatcherTimer::run(
                || {
                    let stopped = SERVER.with(|s| {
                        s.borrow()
                            .as_ref()
                            .is_none_or(|server| !server.is_running())
                    });
                    if stopped {
                        if let Some(lifetime) = ferroui_controls::Application::current()
                            .and_then(|a| a.application_lifetime())
                        {
                            if let Some(lifetime) =
                                lifetime.as_classic_desktop_style_application_lifetime()
                            {
                                lifetime.shutdown(0);
                            }
                        }
                    }
                    !stopped
                },
                std::time::Duration::from_millis(100),
                ferroui_base::threading::DispatcherPriority::BACKGROUND,
            );
        }
        if let Some(lifetime) = builder.instance().and_then(|a| a.application_lifetime()) {
            if let Some(lifetime) = lifetime.as_controlled_application_lifetime() {
                lifetime.exit(Rc::new(|_| {
                    SERVER.with(|s| s.borrow_mut().take());
                }));
            }
        }
    })
}
