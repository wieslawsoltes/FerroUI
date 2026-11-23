//! Ordinary FerroUI composition for leased external textures.
#![forbid(unsafe_code)]
mod renderer_host;
mod texture_presenter;
pub use renderer_host::RendererHost;
pub use texture_presenter::*;
// Shared by runtime registration and the build-time XAML metadata scanner.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_media", "FerroUI.Media"),
    ("ferroui_media_controls", "FerroUI.Media.Controls"),
];
const TYPES: &[&ferroui_base::TypeInfo] = &[TexturePresenter::TYPE, RendererHost::TYPE];
/// Registers the namespaces and the table of types of this crate (and of the crates it is built
/// on). Cheap and idempotent. The table is trimmed as that of every crate of the framework
/// (`TypeInfo::register_table`): a class is known once it is used, and by name where the tables
/// of types are kept.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        ferroui_base::TypeInfo::register_namespaces(NAMESPACES);
        ferroui_base::TypeInfo::register_table(TYPES);
    });
}
