//! Exports ordinary source-scanned metadata without loading a control library.
fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
