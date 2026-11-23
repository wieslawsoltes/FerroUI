// Quiet mode changes sample diagnostics only; failures and stress measurements
// remain visible. Read once before any windows/native workers start.
static QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
macro_rules! println {
    ($($arg:tt)*) => { if !crate::QUIET.load(std::sync::atomic::Ordering::Relaxed) { std::println!($($arg)*); } };
}
#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
mod desktop;
// `compiled_markup`: the table of the units of the WinUI theme the build wrote (`build.rs`).
#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
ferroui_markup_xaml::include_compiled_xaml!();
fn main() -> std::process::ExitCode {
    QUIET.store(
        std::env::args().any(|a| a == "--quiet-diagnostics"),
        std::sync::atomic::Ordering::Relaxed,
    );
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
#[cfg(all(windows, feature = "memory-audit"))]
mod memory_audit;
#[cfg(all(target_os = "macos", feature = "macos-verification"))]
mod macos_allocations;
