use ferroui_controls::AppBuilder;

fn main() -> std::process::ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(index) = args.iter().position(|arg| arg == "--project") {
        let Some(path) = args.get(index + 1) else {
            eprintln!("--project requires a folder or Cargo.toml path");
            return std::process::ExitCode::FAILURE;
        };
        designer_studio::set_initial_project(path.into());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        eprintln!(
            "FerroUI Designer Studio\n  --file PATH        Open a XAML document\n  --project PATH     Open a solution or project folder\n  --mcp              Enable authenticated loopback MCP (read only)\n  --mcp-port PORT    Choose the MCP HTTP port (0 chooses a free port)\n  --mcp-stdio        Use the existing MCP stdio transport\n  --mcp-write        Allow document editing through MCP\n  --mcp-input        Allow input simulation; with --mcp-write, allow behavior and process execution\n  --headless         Use the headless host (requires feature headless)"
        );
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
                Ok(path) => match document_uri(&path) {
                    Ok(uri) => designer_studio::set_initial_document(source, uri),
                    Err(error) => {
                        eprintln!("{}: {error}", path.display());
                        return std::process::ExitCode::FAILURE;
                    }
                },
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

fn document_uri(path: &std::path::Path) -> Result<String, &'static str> {
    let path = path
        .to_str()
        .ok_or("The XAML path contains unsupported non-Unicode characters")?;
    file_uri(path, cfg!(windows)).ok_or("The XAML path could not be represented as a file URI")
}

fn file_uri(path: &str, windows: bool) -> Option<String> {
    use ferroui_base::platform::storage::file_io::StorageProviderHelpers;

    let normalized_unc;
    let path = if windows {
        if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
            normalized_unc = format!(r"\\{unc}");
            &normalized_unc
        } else {
            path
        }
    } else {
        path
    };
    if !windows && path.contains('\\') {
        // Backslashes are literal filename characters on Unix. Escape each piece
        // through the storage helper, which otherwise treats them as separators.
        let mut pieces = path.split('\\');
        let mut uri =
            StorageProviderHelpers::try_get_uri_from_file_path(pieces.next()?, false)?.to_string();
        for piece in pieces {
            let encoded =
                StorageProviderHelpers::try_get_uri_from_file_path(&format!("/{piece}"), false)?;
            uri.push_str("%5C");
            uri.push_str(encoded.absolute_path().strip_prefix('/')?);
        }
        return Some(uri);
    }
    let uri = StorageProviderHelpers::try_get_uri_from_file_path(path, false)?.to_string();
    if windows && path.starts_with(r"\\") && !path.starts_with(r"\\?\") {
        // Keep the UNC server in the authority so relative resource resolution
        // cannot turn the network path into a local path.
        return Some(format!("file://{}", uri.strip_prefix("file:////")?));
    }
    Some(uri)
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
    let Some(mut options) = mcp_options(args)? else {
        return Ok(());
    };
    if matches!(options.transport, ferroui_designer::mcp::Transport::Stdio) {
        // Reserve protocol stdout before platform/theme initialization can write diagnostics.
        options.stdio_output =
            Some(ferroui_designer::mcp::reserve_stdout().map_err(|error| error.to_string())?);
    }
    designer_studio::set_mcp_options(options);
    Ok(())
}

/// Parse transport and permission choices without starting a server or reserving stdout.
#[cfg(feature = "mcp")]
fn mcp_options(args: &[String]) -> Result<Option<ferroui_designer::mcp::Options>, String> {
    use ferroui_designer::mcp::{Options, Transport};
    let enabled = args
        .iter()
        .any(|arg| matches!(arg.as_str(), "--mcp" | "--mcp-port" | "--mcp-stdio"));
    let write = args.iter().any(|arg| arg == "--mcp-write");
    let input = args.iter().any(|arg| arg == "--mcp-input");
    if !enabled {
        if write || input {
            let flag = if write { "--mcp-write" } else { "--mcp-input" };
            return Err(format!("{flag} requires --mcp, --mcp-port, or --mcp-stdio"));
        }
        return Ok(None);
    }
    let stdio = args.iter().any(|arg| arg == "--mcp-stdio");
    let mut options = Options::default();
    options.permissions.allow_property_writes = write;
    options.permissions.allow_input = input;
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
    }
    Ok(Some(options))
}

#[cfg(test)]
mod tests {
    use super::file_uri;
    use ferroui_base::utilities::{Uri, UriKind};

    fn resolve_resource(base: &str) -> String {
        Uri::combine(
            &Uri::absolute(base).unwrap(),
            &Uri::new("Assets/icon.png", UriKind::Relative).unwrap(),
        )
        .to_string()
    }

    #[test]
    fn windows_drive_paths_keep_the_drive_and_resource_directory() {
        for path in [
            r"C:\Designer Studio\Main.xaml",
            r"\\?\C:\Designer Studio\Main.xaml",
        ] {
            let uri = file_uri(path, true).unwrap();
            assert_eq!(uri, "file:///C:/Designer%20Studio/Main.xaml");
            assert_eq!(
                resolve_resource(&uri),
                "file:///C:/Designer%20Studio/Assets/icon.png"
            );
        }
    }

    #[test]
    fn unc_paths_keep_the_network_authority_for_relative_resources() {
        for path in [
            r"\\server\share\Designer Studio\Main.xaml",
            r"\\?\UNC\server\share\Designer Studio\Main.xaml",
        ] {
            let uri = file_uri(path, true).unwrap();
            assert_eq!(uri, "file://server/share/Designer%20Studio/Main.xaml");
            assert_eq!(
                resolve_resource(&uri),
                "file://server/share/Designer%20Studio/Assets/icon.png"
            );
        }
    }

    #[test]
    fn file_paths_escape_literal_percent_unicode_and_uri_delimiters() {
        for (path, windows, prefix) in [
            (r"C:\Designs\żółć %20 [draft]#?.xaml", true, "file:///C:"),
            ("/Designs/żółć %20 [draft]#?.xaml", false, "file://"),
        ] {
            assert_eq!(
                file_uri(path, windows).unwrap(),
                format!(
                    "{prefix}/Designs/%C5%BC%C3%B3%C5%82%C4%87%20%2520%20%5Bdraft%5D%23%3F.xaml"
                )
            );
        }
    }

    #[test]
    fn unix_backslashes_remain_literal_filename_characters() {
        let path = r"/Designs/part\%5C ż.xaml";
        let uri = file_uri(path, false).unwrap();
        assert_eq!(uri, "file:///Designs/part%5C%255C%20%C5%BC.xaml");
        assert_eq!(Uri::absolute(&uri).unwrap().local_path(), path);
    }

    #[cfg(feature = "mcp")]
    fn options(args: &[&str]) -> Result<Option<ferroui_designer::mcp::Options>, String> {
        super::mcp_options(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_execution_permissions_are_explicit_and_independent() {
        assert!(options(&[]).unwrap().is_none());
        for (flags, writes, input) in [
            (vec!["--mcp"], false, false),
            (vec!["--mcp", "--mcp-write"], true, false),
            (vec!["--mcp", "--mcp-input"], false, true),
            (vec!["--mcp", "--mcp-write", "--mcp-input"], true, true),
        ] {
            let options = options(&flags).unwrap().unwrap();
            assert_eq!(options.permissions.allow_property_writes, writes);
            assert_eq!(options.permissions.allow_input, input);
            assert!(options.stdio_output.is_none());
        }
        for flag in ["--mcp-write", "--mcp-input"] {
            assert!(options(&[flag]).err().unwrap().contains("requires --mcp"));
        }
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_input_opt_in_keeps_transport_validation_and_stdio_reservation_order() {
        use ferroui_designer::mcp::Transport;
        let http = options(&["--mcp-port", "0", "--mcp-input"])
            .unwrap()
            .unwrap();
        assert!(
            matches!(http.transport, Transport::Http(address) if address.ip().is_loopback() && address.port() == 0)
        );
        assert!(http.permissions.allow_input);
        let stdio = options(&["--mcp-stdio", "--mcp-write", "--mcp-input"])
            .unwrap()
            .unwrap();
        assert!(matches!(stdio.transport, Transport::Stdio));
        assert!(stdio.permissions.allow_property_writes && stdio.permissions.allow_input);
        // Parsing must not redirect the test runner or reserve process-wide protocol output.
        assert!(stdio.stdio_output.is_none());
        assert!(options(&["--mcp-stdio", "--mcp-port", "0", "--mcp-input"]).is_err());
        assert!(options(&["--mcp-port", "--mcp-input"]).is_err());
        assert!(options(&["--mcp-port", "65536", "--mcp-input"]).is_err());
    }
}
