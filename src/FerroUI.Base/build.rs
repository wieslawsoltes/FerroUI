//! Exports the type model of the crate: its sources are scanned as files, and what
//! their declarations state (the classes and their registered properties, the markup
//! metadata, the enumerations, the namespaces, the assembly) is written to
//! `$OUT_DIR/ferroui_base.xamlmeta`, which the markup compiler of a crate that depends on this
//! one resolves the types of its documents against without linking this crate
//! (docs/porting/xaml.md, 9.5.13 and 9.6.3). Nothing the crate compiles is generated here.
//!
//! The script runs again only when a source file of the crate, its manifest or this file
//! changes; the model is written only when its text changed. What a run cost is in
//! `$OUT_DIR/xamlmeta-scan.txt`.

fn main() {
    ferroui_build_scan::export::Export::from_env().run();
}
