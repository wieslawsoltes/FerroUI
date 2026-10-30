//! Exports the original markup extension metadata for independent consumers.
fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
