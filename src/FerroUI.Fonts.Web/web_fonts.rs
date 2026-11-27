//! Declaring faces in markup.

use crate::WebFontCollection;
use ferroui_base::platform::IFontManagerImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, AttachedProperty, FerroLocator,
    FerroObject, FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, LocatorExtensions, Ref,
};

/// Declares web fonts from markup, with the `@font-face` rules of a style sheet, which is the
/// standard form of every descriptor:
///
/// ```xml
/// <Application>
///   <WebFonts.StyleSheet>
///     @font-face {
///       font-family: Inter;
///       src: url(/fonts/Inter.woff2) format(woff2);
///       font-weight: 100 900;
///       font-display: swap;
///     }
///   </WebFonts.StyleSheet>
/// </Application>
/// ```
///
/// The rules join [`WebFontCollection::current`], and text names the family as
/// `fonts:WebFonts#Inter`. Nothing is fetched until text needs it. A rule that is not valid is
/// dropped, as CSS does.
#[repr(C)]
pub struct WebFonts {
    base: FerroObject,
}

ferro_class!(WebFonts: FerroObject);

ferro_class_info!(WebFonts {
    new: WebFonts::new,
    markup: {
        namespace: "FerroUI.Fonts.Web",
    },
});

ferro_impl_classes!(WebFonts: FerroObjectImpl);

ferro_properties! { impl WebFonts {
    pub fn style_sheet_property() -> AttachedProperty<String> {
        FerroProperty::register_attached::<WebFonts, FerroObject, _>("StyleSheet", String::new())
    }
} }

impl WebFonts {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn get_style_sheet(element: &FerroObject) -> String {
        element.get_value(Self::style_sheet_property())
    }

    pub fn set_style_sheet(element: &FerroObject, value: impl Into<String>) {
        element.set_value(Self::style_sheet_property(), value.into());
    }

    /// Starts following the property. Run by `register_types` and `with_web_fonts`, once.
    pub(crate) fn follow_style_sheets() {
        ferroui_base::ferro_thread_local! {
            static FOLLOWING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        }
        if !FOLLOWING.with(|following| following.replace(true)) {
            Self::style_sheet_property().changed().subscribe(Self::style_sheet_changed).detach();
        }
    }

    fn style_sheet_changed(change: &FerroPropertyChangedEventArgs<'_>) {
        // Without a font backend there is nothing to declare a face on (a designer, a test).
        if FerroLocator::current().get_service::<dyn IFontManagerImpl>().is_none() {
            return;
        }
        WebFontCollection::current().add_style_sheet(&change.get_new_value::<String>());
    }
}
