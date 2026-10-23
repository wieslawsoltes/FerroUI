//! Exports the type model of the crate: its sources are scanned as files, and what their
//! declarations state (the classes and their registered properties, the namespace, the
//! assembly) is written to `$OUT_DIR/ferroui_view.xamlmeta`, which the markup compiler of a
//! crate that depends on this one resolves `ComponentHost` against (docs/porting/xaml.md,
//! 9.5.13 and 9.6.3). Nothing the crate compiles is generated here.

fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
