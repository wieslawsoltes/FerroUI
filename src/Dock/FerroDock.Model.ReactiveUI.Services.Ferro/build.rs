//! Exports this library's class metadata to dependent compiled markup builds.
fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
