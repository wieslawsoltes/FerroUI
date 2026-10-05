use crate::AppBuilder;
use ferroui_base::media::fonts::{FontCollectionBase, SystemFontCollection};
use ferroui_base::utilities::Uri;

/// The system font extension of the application builder.
impl AppBuilder {
    /// Adds `font_source` to the system fonts once the application is set
    /// up: the fonts of the source become available as if they were
    /// installed in the system.
    pub fn with_system_font_source(&self, font_source: Uri) -> AppBuilder {
        self.configure_fonts(move |font_manager| {
            let system_fonts = font_manager.system_fonts();
            if let Some(system_font_collection) = system_fonts.as_any().downcast_ref::<SystemFontCollection>() {
                FontCollectionBase::try_add_font_source(system_font_collection, &font_source);
            }
        })
    }
}
