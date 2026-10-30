//! Exports the source class metadata through the shared compiler scanner.
fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
