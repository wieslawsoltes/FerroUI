//! Video-specific UI, layered on the common media texture presenter.
#![forbid(unsafe_code)]
pub use ferroui_media_controls::{TextureFrameImporter, TexturePresenter};
mod media_player_element;
mod media_transport_controls;
pub use media_player_element::*;
pub use media_transport_controls::*;
pub fn register_types() {
    ferroui_media_controls::register_types();
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroui_media_video_controls",
        "FerroUI.Media.Video.Controls",
    )]);
    ferroui_base::TypeInfo::register(MediaPlayerElement::TYPE);
    ferroui_base::TypeInfo::register(MediaTransportControls::TYPE);
}
