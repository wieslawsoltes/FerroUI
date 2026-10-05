use std::rc::Rc;

use crate::media::text_formatting::GenericTextRunProperties;
use crate::media::{
    BaselineAlignment, FontFeatureCollection, GlyphTypeface, IBrush, TextDecorationCollection, Typeface,
};
use crate::utilities::CultureInfo;

/// Properties that can change from one run to the next, such as typeface or
/// foreground brush.
///
/// Handled as `Rc<dyn TextRunProperties>`; compare with `==` on the trait
/// object (value equality of the members, as upstream's `Equals`).
pub trait TextRunProperties: 'static {
    /// Run typeface.
    fn typeface(&self) -> &Typeface;

    /// Em size of font used to format and display text.
    fn font_rendering_em_size(&self) -> f64;

    /// Run text decorations.
    fn text_decorations(&self) -> Option<&TextDecorationCollection>;

    /// Brush used to fill text.
    fn foreground_brush(&self) -> Option<&Rc<dyn IBrush>>;

    /// Brush used to paint background of run.
    fn background_brush(&self) -> Option<&Rc<dyn IBrush>>;

    /// Run text culture.
    fn culture_info(&self) -> Option<&CultureInfo>;

    /// Optional features of used font.
    fn font_features(&self) -> Option<&FontFeatureCollection> {
        None
    }

    /// Run vertical box alignment.
    fn baseline_alignment(&self) -> BaselineAlignment {
        BaselineAlignment::Baseline
    }

    /// The glyph typeface of [`TextRunProperties::typeface`].
    ///
    /// Upstream caches it in a field of the base class; implementations that
    /// are used for more than a handful of runs should do the same (as
    /// [`GenericTextRunProperties`] does) and override this method.
    fn cached_glyph_typeface(&self) -> Rc<GlyphTypeface> {
        self.typeface().glyph_typeface()
    }
}

fn rc_option_eq<T: PartialEq + ?Sized>(a: Option<&Rc<T>>, b: Option<&Rc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b) || **a == **b,
        _ => false,
    }
}

impl PartialEq for dyn TextRunProperties {
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::addr_eq(self as *const dyn TextRunProperties, other as *const dyn TextRunProperties) {
            return true;
        }

        self.typeface() == other.typeface()
            && self.font_rendering_em_size() == other.font_rendering_em_size()
            && self.text_decorations() == other.text_decorations()
            && rc_option_eq(self.foreground_brush(), other.foreground_brush())
            && rc_option_eq(self.background_brush(), other.background_brush())
            && self.culture_info() == other.culture_info()
            && self.font_features() == other.font_features()
    }
}

impl dyn TextRunProperties {
    /// The same properties with another typeface.
    pub(crate) fn with_typeface(self: &Rc<Self>, typeface: &Typeface) -> Rc<dyn TextRunProperties> {
        if self.typeface() == typeface {
            return self.clone();
        }

        Rc::new(GenericTextRunProperties::with_all(
            typeface.clone(),
            self.font_rendering_em_size(),
            self.text_decorations().cloned(),
            self.foreground_brush().cloned(),
            self.background_brush().cloned(),
            self.baseline_alignment(),
            self.culture_info().cloned(),
            self.font_features().cloned(),
        ))
    }
}
