//! States the features of the crate to the build scripts of the crates that depend on it: the
//! generator of their locators reads `DEP_FERROUI_STATIC_VIEW_LOCATOR_REACTIVEUI` to report a
//! locator that asks for the view locator of ReactiveUI without the feature `reactiveui` (SVL0001).

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_REACTIVEUI").is_some() {
        println!("cargo::metadata=reactiveui=true");
    }
}
