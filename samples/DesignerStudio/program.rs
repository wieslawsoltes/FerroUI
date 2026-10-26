use ferroui_controls::AppBuilder;

fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        eprintln!("FerroUI Designer Studio\n  --file PATH        Open a XAML document\n  --mcp              Enable authenticated loopback MCP (read only)\n  --mcp-port PORT    Choose the MCP HTTP port (0 chooses a free port)\n  --mcp-stdio        Use the existing MCP stdio transport\n  --mcp-write        Allow document editing through MCP\n  --headless         Use the headless host (requires feature headless)");
        return std::process::ExitCode::SUCCESS;
    }
    #[cfg(feature = "mcp")]
    if let Err(error) = configure_mcp(&args) {
        eprintln!("{error}");
        return std::process::ExitCode::FAILURE;
    }
    #[cfg(not(feature = "mcp"))]
    if args.iter().any(|arg| arg.starts_with("--mcp")) {
        eprintln!("Build with feature mcp to use MCP options");
        return std::process::ExitCode::FAILURE;
    }
    if let Some(index) = args.iter().position(|arg| arg == "--file") {
        let Some(path) = args.get(index + 1) else {
            eprintln!("--file requires a XAML path");
            return std::process::ExitCode::FAILURE;
        };
        match std::fs::read_to_string(path) {
            Ok(source) => match std::fs::canonicalize(path) {
                Ok(path) => {
                    let mut path = path.to_string_lossy().replace('\\', "/");
                    if !path.starts_with('/') {
                        path.insert(0, '/');
                    }
                    let path = path
                        .replace('%', "%25")
                        .replace(' ', "%20")
                        .replace('#', "%23")
                        .replace('?', "%3F");
                    designer_studio::set_initial_document(source, format!("file://{path}"));
                }
                Err(error) => {
                    eprintln!("{path}: {error}");
                    return std::process::ExitCode::FAILURE;
                }
            },
            Err(error) => {
                eprintln!("{path}: {error}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
    let builder = AppBuilder::configure::<designer_studio::App>();
    #[cfg(feature = "headless")]
    let builder = if args.iter().any(|arg| arg == "--headless") {
        use ferroui_headless::FerroHeadlessPlatformExtensions;
        builder.use_headless(ferroui_headless::FerroHeadlessPlatformOptions::default())
    } else {
        platform(builder)
    };
    #[cfg(not(feature = "headless"))]
    let builder = platform(builder);
    let code = builder
        .after_setup(|_| sample_support::smoke_run())
        .start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(code as u8)
}
#[cfg(feature = "desktop")]
fn platform(builder: AppBuilder) -> AppBuilder {
    use ferroui_desktop::AppBuilderDesktopExtensions;
    builder.use_platform_detect()
}
#[cfg(not(feature = "desktop"))]
fn platform(builder: AppBuilder) -> AppBuilder {
    builder
}

#[cfg(feature = "mcp")]
fn configure_mcp(args: &[String]) -> Result<(), String> {
    use ferroui_designer::mcp::{Options, Transport};
    let enabled = args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--mcp" | "--mcp-port" | "--mcp-stdio"));
    let write = args.iter().any(|arg| arg == "--mcp-write");
    if !enabled {
        if write {
            return Err("--mcp-write requires --mcp, --mcp-port, or --mcp-stdio".into());
        }
        return Ok(());
    }
    let stdio = args.iter().any(|arg| arg == "--mcp-stdio");
    let mut options = Options::default();
    options.permissions.allow_property_writes = write;
    if let Some(index) = args.iter().position(|arg| arg == "--mcp-port") {
        if stdio {
            return Err("Choose either --mcp-port or --mcp-stdio".into());
        }
        let port = args
            .get(index + 1)
            .ok_or("--mcp-port requires a port number")?
            .parse::<u16>()
            .map_err(|_| "MCP port must be an integer from 0 to 65535")?;
        options.transport = Transport::Http(std::net::SocketAddr::from(([127, 0, 0, 1], port)));
    }
    if stdio {
        options.transport = Transport::Stdio;
        // Reserve protocol stdout before platform/theme initialization can write diagnostics.
        options.stdio_output =
            Some(ferroui_designer::mcp::reserve_stdout().map_err(|error| error.to_string())?);
    }
    designer_studio::set_mcp_options(options);
    Ok(())
}
