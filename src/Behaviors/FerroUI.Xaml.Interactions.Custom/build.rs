//! Exports the type model of the crate for the markup compiler of the crates that depend on it
//! (docs/porting/xaml.md, 9.5.13 and 9.6.3).

fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
