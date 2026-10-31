#[cfg(target_os = "macos")]
mod desktop;
fn main() -> std::process::ExitCode {
    #[cfg(target_os = "macos")]
    {
        desktop::run()
    }
    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("This native sample currently has an Apple provider; other native providers are not implemented yet.");
        std::process::ExitCode::FAILURE
    }
}
