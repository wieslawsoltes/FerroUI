fn main() {
    #[cfg(feature = "app-preview")]
    ferroui_build_scan::export::Export::from_env()
        .source_root("main.rs")
        .run();
}
