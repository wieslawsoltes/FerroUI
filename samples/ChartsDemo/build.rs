//! Compile the same declarative Charts surface that the runtime-loader tests use. The desktop
//! application constructs the WinUI theme in code (`app.rs`) from the units it needs
//! (docs/xaml-themes.md, section 3): with the feature `desktop` the build writes the table of
//! those units, from the roots of the documents and of the sources.
use ferroui_build::{Build, TypeSystem, XamlGroup};

fn main() {
    println!("cargo::rerun-if-changed=MainView.xaml");
    let mut build = Build::from_env().type_system(TypeSystem::Model).compile_group(
        XamlGroup::new("compiled_view").documents(&[("MainView.xaml", include_str!("MainView.xaml"))]),
    );
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        build = build.theme("WinUITheme");
    }
    build.run();
}
