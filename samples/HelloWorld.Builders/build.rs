//! The units of the WinUI theme the sample needs (docs/xaml-themes.md, section 3): the build reads
//! the sources of the crate, finds the controls its builders make (`button()`, `text_block()`), and
//! writes the table `main.rs` constructs the theme with.

fn main() {
    ferroui_build::Build::from_env().type_system(ferroui_build::TypeSystem::Model).theme("WinUITheme").run();
}
