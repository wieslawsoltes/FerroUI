use crate::AppBuilder;
use ferroui_base::media::fonts::{FontCollectionBase, SystemFontCollection};

/// The system font extension of the application builder.
impl AppBuilder {
    /// Adds `font_source` to the system fonts once the application is set
    /// up: the fonts of the source become available as if they were
    /// installed in the system. The source is a `file:` URL of a font file or
    /// of a directory of font files, or the URL of font assets (`ferres:`,
    /// `resm:`).
    pub fn with_system_font_source(&self, font_source: url::Url) -> AppBuilder {
        self.configure_fonts(move |font_manager| {
            let system_fonts = font_manager.system_fonts();
            if let Some(system_font_collection) = system_fonts.as_any().downcast_ref::<SystemFontCollection>() {
                FontCollectionBase::try_add_font_source(system_font_collection, &font_source);
            }
        })
    }
}
