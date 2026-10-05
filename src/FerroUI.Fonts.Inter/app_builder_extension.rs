use crate::InterFontCollection;
use ferroui_controls::AppBuilder;
use std::rc::Rc;

/// The Inter font extension of the application builder.
pub trait AppBuilderExtension {
    /// Adds the embedded Inter family to the font manager once the
    /// application is set up.
    fn with_inter_font(&self) -> AppBuilder;
}

impl AppBuilderExtension for AppBuilder {
    fn with_inter_font(&self) -> AppBuilder {
        self.configure_fonts(|font_manager| {
            font_manager.add_font_collection(Rc::new(InterFontCollection::new()));
        })
    }
}
