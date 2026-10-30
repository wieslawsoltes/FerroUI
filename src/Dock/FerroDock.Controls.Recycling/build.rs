//! Exports native class/property/contract metadata for downstream markup.
fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
