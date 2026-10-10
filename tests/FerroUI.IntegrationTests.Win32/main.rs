//! Upstream's integration tests of the Windows platform backend
//! (`tests/*.IntegrationTests.Win32` of the reference): windows of the
//! window class over the backend, on a live desktop, compared with what
//! the system reports of them.
//!
//! ```text
//! cargo test -p ferroui-integration-tests-win32 --test integration
//! cargo test -p ferroui-integration-tests-win32 --test integration -- full_screen
//! ```
//!
//! The binary has no test harness: the tests open windows and run the
//! message loop of the thread that set the application up, so they run one
//! after the other on the main thread. Every test is printed with its
//! result as the harness prints it; an argument selects the tests whose
//! name contains it. The names are the names of upstream's tests in snake
//! case, a theory with its arguments.
//!
//! The tests need a desktop. On a system that is not Windows the binary
//! compiles and runs nothing.

mod begin_move_drag_tests;
mod infrastructure;
mod standard_window_tests;
mod unmanaged_methods;
mod window_extensions;

use ferroui_base::Ref;
use ferroui_controls::Window;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::ExitCode;

/// A test: its name and what it runs.
pub struct TestCase {
    name: String,
    run: Box<dyn Fn()>,
}

impl TestCase {
    pub fn new(name: String, run: impl Fn() + 'static) -> Self {
        Self { name, run: Box::new(run) }
    }
}

/// Closes the window of a test when the test ends, however it ends:
/// `Dispose` of upstream's test classes.
pub struct WindowGuard(pub Ref<Window>);

impl Drop for WindowGuard {
    fn drop(&mut self) {
        self.0.close();
    }
}

fn main() -> ExitCode {
    let filters: Vec<String> = std::env::args().skip(1).filter(|argument| !argument.starts_with('-')).collect();

    if !cfg!(windows) {
        println!("the integration tests of the Windows platform backend run on Windows only: nothing to run");
        return ExitCode::SUCCESS;
    }

    infrastructure::app_manager::ensure_app_initialized();

    let mut cases = Vec::new();
    begin_move_drag_tests::tests(&mut cases);
    standard_window_tests::tests(&mut cases);

    let selected: Vec<&TestCase> = cases
        .iter()
        .filter(|case| filters.is_empty() || filters.iter().any(|filter| case.name.contains(filter.as_str())))
        .collect();

    println!("\nrunning {} tests", selected.len());
    let mut failed = Vec::new();
    for case in &selected {
        // A test that fails panics; its window is closed by its guard.
        let result = catch_unwind(AssertUnwindSafe(|| (case.run)()));
        match result {
            Ok(()) => println!("test {} ... ok", case.name),
            Err(_) => {
                println!("test {} ... FAILED", case.name);
                failed.push(case.name.as_str());
            }
        }
    }

    if !failed.is_empty() {
        println!("\nfailures:");
        for name in &failed {
            println!("    {name}");
        }
    }
    println!(
        "\ntest result: {}. {} passed; {} failed; {} filtered out",
        if failed.is_empty() { "ok" } else { "FAILED" },
        selected.len() - failed.len(),
        failed.len(),
        cases.len() - selected.len()
    );

    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
