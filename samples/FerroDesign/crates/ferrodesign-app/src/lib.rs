#![forbid(unsafe_code)]
extern crate self as ferrodesign_app;
pub mod app;
pub mod canvas;
#[cfg(not(target_os = "emscripten"))]
pub mod desktop;
pub mod icons;
pub mod platform;
pub mod prototype;
pub mod renderer;
pub mod shell;

use ferroui_base::metadata::MarkupAssembly;
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDesign",
    crate_name: "ferrodesign_app",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_markup_xaml::include_compiled_xaml!();
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_themes_fluent::register_types();
        MarkupAssembly::register(&ASSEMBLY);
        compiled_markup::register();
    });
}
