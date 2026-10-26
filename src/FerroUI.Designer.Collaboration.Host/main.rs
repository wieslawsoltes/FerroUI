use ferroui_designer_collaboration_host::{load_private, save_private, HostedServer, Secret};
use std::path::PathBuf;
fn main() {
    if let Err(error) = run() {
        eprintln!("FerroUI collaboration: {error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut storage = None;
    let mut bind = "127.0.0.1:7491".parse()?;
    let mut public = None;
    let mut endpoint_file = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--storage" => {
                storage = Some(PathBuf::from(
                    args.next().ok_or("--storage requires a directory")?,
                ))
            }
            "--bind" => {
                bind = args
                    .next()
                    .ok_or("--bind requires a loopback address")?
                    .parse()?
            }
            "--public-url" => {
                public = Some(args.next().ok_or("--public-url requires an HTTPS URL")?)
            }
            "--endpoint-file" => {
                endpoint_file = Some(PathBuf::from(
                    args.next().ok_or("--endpoint-file requires a path")?,
                ))
            }
            "--help" | "-h" => {
                println!("ferroui-collaboration-server --storage DIRECTORY [--bind 127.0.0.1:7491] [--public-url https://host/mcp] [--endpoint-file PATH]\nCredentials are created with owner-only permissions in DIRECTORY. Serve remote peers through a TLS reverse proxy; retain the internal loopback Host header.");
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}").into()),
        }
    }
    let storage = storage.ok_or("--storage is required")?;
    std::fs::create_dir_all(&storage)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&storage, std::fs::Permissions::from_mode(0o700))?;
    }
    let admin_path = storage.join("administrator.json");
    let admin: Secret = if admin_path.exists() {
        load_private(&admin_path)?
    } else {
        let secret = Secret::generate()?;
        save_private(&admin_path, &secret)?;
        secret
    };
    let server = HostedServer::start(&storage, &admin, Some(bind))?;
    let mut endpoint = server.endpoint();
    if let Some(public) = public {
        if !public.starts_with("https://") {
            return Err("--public-url must use HTTPS".into());
        }
        endpoint.url = public;
    }
    let endpoint_file = endpoint_file.unwrap_or_else(|| storage.join("endpoint.json"));
    save_private(&endpoint_file, &endpoint)?;
    eprintln!("FerroUI collaboration listening on {bind}; endpoint credentials: {}; administrator credential: {}",endpoint_file.display(),admin_path.display());
    while server.is_running() {
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Ok(())
}
