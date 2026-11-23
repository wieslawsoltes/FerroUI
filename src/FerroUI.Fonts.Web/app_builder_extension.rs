use crate::WebFontCollection;
use ferroui_controls::AppBuilder;
use std::rc::Rc;

/// The web fonts extension of the application builder.
pub trait AppBuilderExtension {
    /// Registers the web fonts of the application once it is set up. `configure` declares its
    /// faces; nothing is fetched until text needs them.
    fn with_web_fonts(&self, configure: impl Fn(&WebFontCollection) + 'static) -> AppBuilder;
}

impl AppBuilderExtension for AppBuilder {
    fn with_web_fonts(&self, configure: impl Fn(&WebFontCollection) + 'static) -> AppBuilder {
        crate::WebFonts::follow_style_sheets();
        let configure = Rc::new(configure);
        self.configure_fonts(move |_| configure(&WebFontCollection::current()))
    }
}
