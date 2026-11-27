//! Generates the trimmed WinUI theme part table for the controls used in `physics-sandbox`.

fn main() {
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .theme("WinUITheme")
        .run();
}
