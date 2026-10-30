fn main() {
    // The type model of the crate, for the build scripts of the crates whose markup names its
    // types (an application's compiled XAML): Cargo hands them its path through `links`.
    ferroui_build_scan::export::Export::from_env().run();
}
