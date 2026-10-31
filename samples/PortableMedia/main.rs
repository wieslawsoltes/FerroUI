#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
mod desktop;
fn main() -> std::process::ExitCode {
    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    {
        desktop::run()
    }
    #[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
    {
        eprintln!("Use the separate mobile or browser sample on this target.");
        std::process::ExitCode::FAILURE
    }
}
