#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod services;
#[cfg(target_os = "linux")]
mod session;
#[cfg(target_os = "linux")]
mod shell;

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    linux::run()
}

#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("Ferro Desktop requires Linux and an X11 display.");
    std::process::ExitCode::FAILURE
}
