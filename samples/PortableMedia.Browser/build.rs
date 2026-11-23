//! The units of the Fluent theme the browser application needs (docs/xaml-themes.md, section 3):
//! the build reads the sources of the crate (`main.rs` and `demo.rs`), finds the controls they make,
//! and writes the table `main.rs` constructs the theme with.

fn main() {
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .source_root("main.rs")
        .theme("FluentTheme")
        .run();
}
