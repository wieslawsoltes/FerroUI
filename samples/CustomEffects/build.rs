//! The units of the WinUI theme the application of the gallery needs (docs/xaml-themes.md, section
//! 3): with the feature `app` the build reads the sources of the crate, finds the controls the
//! gallery makes, and writes the table `main.rs` constructs the theme with. Without it the crate
//! is the sections of the gallery, which constructs no theme.

fn main() {
    if std::env::var_os("CARGO_FEATURE_APP").is_none() {
        println!("cargo::rerun-if-changed=build.rs");
        return;
    }
    ferroui_build::Build::from_env().type_system(ferroui_build::TypeSystem::Model).theme("WinUITheme").run();
}
