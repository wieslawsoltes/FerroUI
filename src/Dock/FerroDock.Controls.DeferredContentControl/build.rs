//! Exports native dependency metadata using the shared scanner.
fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
