//! Video-specific UI, layered on the common media texture presenter.
#![forbid(unsafe_code)]
pub mod automation;
pub use ferroui_media_controls::{TextureFrameImporter, TexturePresenter};
mod full_window;
mod media_player_element;
mod media_player_presenter;
mod media_transport_controls;
pub use media_player_element::*;
pub use media_player_presenter::*;
pub use media_transport_controls::*;
// Shared by runtime registration and the build-time XAML metadata scanner.
const NAMESPACES: &[(&str, &str)] = &[(
    "ferroui_media_video_controls",
    "FerroUI.Media.Video.Controls",
)];
pub static ASSEMBLY: ferroui_base::metadata::MarkupAssembly =
    ferroui_base::metadata::MarkupAssembly {
        name: "FerroUI.Media.Video.Controls",
        crate_name: "ferroui_media_video_controls",
        xmlns_definitions: &[ferroui_base::metadata::XmlnsDefinition {
            xml_namespace: ferroui_base::metadata::FERRO_XML_NAMESPACE,
            namespace: "FerroUI.Media.Video.Controls",
        }],
        xmlns_prefixes: &[],
        metadata: &[],
    };
ferroui_base::ferro_rust_paths! {
    classes: [crate::MediaPlayerElement, crate::MediaPlayerPresenter, crate::MediaTransportControls],
    types: [], contracts: [], generics: [], generic_contracts: [],
}
const TYPES: &[&ferroui_base::TypeInfo] = &[
    MediaPlayerElement::TYPE,
    automation::MediaPlayerElementAutomationPeer::TYPE,
    MediaPlayerPresenter::TYPE,
    MediaTransportControls::TYPE,
    automation::MediaTransportControlsAutomationPeer::TYPE,
];
/// Registers the namespaces, the assembly and the table of types of this crate (and of the
/// crates it is built on). Cheap and idempotent. The themes of the WinUI family call it, so the
/// table is trimmed as that of every crate of the framework (`TypeInfo::register_table`): an
/// application that shows no media control links none.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_media_controls::register_types();
        ferroui_base::TypeInfo::register_namespaces(NAMESPACES);
        ferroui_base::metadata::MarkupAssembly::register_table(&ASSEMBLY);
        register_rust_paths();
        ferroui_base::TypeInfo::register_table(TYPES);
    });
}

/// Optional media controls whose control themes belong to the WinUI family.
pub const WINUI_ONLY_CONTROLS: &[(&ferroui_base::TypeInfo, &str)] = &[
    (
        MediaPlayerElement::TYPE,
        "Portable media UI styled by the WinUI theme and its derived themes",
    ),
    (
        MediaPlayerPresenter::TYPE,
        "Portable media UI styled by the WinUI theme and its derived themes",
    ),
    (
        MediaTransportControls::TYPE,
        "Portable media UI styled by the WinUI theme and its derived themes",
    ),
];
