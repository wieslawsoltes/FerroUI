//! Exports the type model of the crate: its sources are scanned as files, and what
//! their declarations state (the classes and their registered properties, the markup
//! metadata, the enumerations, the namespaces, the assembly) is written to
//! `$OUT_DIR/ferroui_controls.xamlmeta`, which the markup compiler of a crate that depends on this
//! one resolves the types of its documents against without linking this crate
//! (docs/porting/xaml.md, 9.5.13 and 9.6.3). Nothing the crate compiles is generated here.
//!
//! The script runs again only when a source file of the crate, its manifest or this file
//! changes (or the model of the base crate, which the scan resolves the names of that crate with); the model is written only when its text changed. What a run cost is in
//! `$OUT_DIR/xamlmeta-scan.txt`.

// The targets on which no code constructs the drawn decorations of a window (#1002), the table
// the crate keeps in step with its window backends.
#[path = "chrome/drawn_decorations_targets.rs"]
mod drawn_decorations_targets;
// The hosts a popup creates through a factory function the scan does not read (#994).
#[path = "primitives/popup_hosts.rs"]
mod popup_hosts;

fn main() {
    let mut export = ferroui_build_scan::export::Export::from_env()
        .not_created_on(drawn_decorations_targets::CLASS, drawn_decorations_targets::NOT_CREATED_ON);
    for (class, created) in popup_hosts::CREATES {
        export = export.creates(class, created);
    }
    export.run();
}
