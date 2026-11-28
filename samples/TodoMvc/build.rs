//! The units of the WinUI theme the application needs (docs/xaml-themes.md, section 3): the build
//! reads the sources of the crate, finds the controls its components make, and writes the table
//! `shots.rs` constructs the theme with.

fn main() {
    ferroui_build::Build::from_env().type_system(ferroui_build::TypeSystem::Model).theme("WinUITheme").run();
}
