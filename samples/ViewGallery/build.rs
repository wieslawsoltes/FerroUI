//! The units of the Fluent theme the gallery needs (docs/xaml-themes.md, section 3): the build
//! reads the sources of the crate (the library and `main.rs`), finds the controls they make, and
//! writes the table `main.rs` constructs the theme with. The themes of the colour picker and the
//! TreeDataGrid are documents of their crates, loaded whole; what they name roots the units of the
//! Fluent theme they build on.

fn main() {
    ferroui_build::Build::from_env().type_system(ferroui_build::TypeSystem::Model).theme("FluentTheme").run();
}
