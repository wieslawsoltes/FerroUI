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
pub fn register_types() {
    ferroui_controls::register_types();
    ferroui_base::TypeInfo::register_namespaces(NAMESPACES);
    ferroui_base::TypeInfo::register(TexturePresenter::TYPE);
    ferroui_base::TypeInfo::register(RendererHost::TYPE);
}
