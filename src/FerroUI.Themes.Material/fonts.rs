//! Roboto bundled under the SIL Open Font License. Source hashes are recorded in
//! `scripts/material-theme/font-provenance.json`.
use ferroui_base::media::fonts::EmbeddedFontCollection;
use ferroui_base::platform::register_assets;
use ferroui_base::utilities::Uri;
use ferroui_controls::AppBuilder;
use std::rc::Rc;

pub(crate) fn register() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        register_assets(
            crate::ASSEMBLY.name,
            &[
                (
                    "/Assets/Roboto-Regular.ttf",
                    include_bytes!("Assets/Roboto-Regular.ttf"),
                ),
                (
                    "/Assets/Roboto-Medium.ttf",
                    include_bytes!("Assets/Roboto-Medium.ttf"),
                ),
                (
                    "/Assets/Roboto-Bold.ttf",
                    include_bytes!("Assets/Roboto-Bold.ttf"),
                ),
            ],
        )
    });
}

/// Adds the embedded Android font family to FerroUI's cross-platform font manager.
pub trait MaterialAppBuilderExtension {
    fn with_material_fonts(&self) -> AppBuilder;
}

impl MaterialAppBuilderExtension for AppBuilder {
    fn with_material_fonts(&self) -> AppBuilder {
        self.configure_fonts(|manager| {
            register();
            manager.add_font_collection(Rc::new(EmbeddedFontCollection::new(
                Uri::absolute("fonts:MaterialRoboto").expect("font key"),
                Uri::absolute("ferres://FerroUI.Themes.Material/Assets").expect("font asset URI"),
            )));
        })
    }
}
