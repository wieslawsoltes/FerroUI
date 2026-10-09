//! Exports the type model of the crate: its sources are scanned as files, and what
//! their declarations state (the classes and their registered properties, the markup
//! metadata, the enumerations, the namespaces, the assembly) is written to
//! `$OUT_DIR/ferroui_markup_xaml.xamlmeta`, which the markup compiler of a crate that depends on this
//! one resolves the types of its documents against without linking this crate
//! (docs/porting/xaml.md, 9.5.13 and 9.6.3). Nothing the crate compiles is generated here.
//!
//! The script runs again only when a source file of the crate, its manifest or this file
//! changes (or the model of the base crate or of the controls, which the scan resolves the names of those crates with); the model is written only when its text changed. What a run cost is in
//! `$OUT_DIR/xamlmeta-scan.txt`.

fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
