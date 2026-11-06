//! Ordinary FerroUI composition for leased external textures.
#![forbid(unsafe_code)]
mod renderer_host;
mod texture_presenter;
pub use renderer_host::RendererHost;
pub use texture_presenter::*;
pub fn register_types() {
    ferroui_controls::register_types();
    ferroui_base::TypeInfo::register_namespaces(&[
        ("ferroui_media", "FerroUI.Media"),
        ("ferroui_media_controls", "FerroUI.Media.Controls"),
    ]);
    ferroui_base::TypeInfo::register(TexturePresenter::TYPE);
    ferroui_base::TypeInfo::register(RendererHost::TYPE);
}
