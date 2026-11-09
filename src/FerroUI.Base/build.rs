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
//!
//! With the feature `full-themes` it also tells the build scripts of the crates that depend on
//! this one that something in the build can show a control that no compiled document and no
//! source of the application names (`cargo::metadata=full_themes=1`, which they read as
//! `DEP_FERROUI_BASE_XAML_FULL_THEMES` through the `links` key of the manifest): the run-time
//! loader, hot reload, the designer, DevTools, Inspect. The compiled themes then apply every unit
//! to a load by URI, and an application that trims its themes keeps them all (#568, step 3 of
//! docs/porting/theme-trimming.md). The tables of types alone (`type-tables`) do not.

fn main() {
    ferroui_build_scan::export::Export::from_env().run();
    if std::env::var_os("CARGO_FEATURE_FULL_THEMES").is_some() {
        println!("cargo::metadata=full_themes=1");
    }
}
