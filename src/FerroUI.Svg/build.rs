//! Exports the markup metadata of the crate for compiled documents.
fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
