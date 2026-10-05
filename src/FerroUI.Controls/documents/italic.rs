use super::{InlineImpl, Span, TextElementImpl};
use super::TextElement;
use ferroui_base::media::FontStyle;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl,
};

/// `Italic` element - markup helper for indicating italicized content.
/// Equivalent to a `Span` with the `FontStyle` property set to italic. Can
/// contain other inline elements.
#[repr(C)]
pub struct Italic {
    base: Span,
}

ferro_class!(Italic: Span);
ferroui_base::ferro_class_info!(Italic { new: Italic::new });
ferro_impl_classes!(Italic: StyledElementImpl, TextElementImpl, InlineImpl);

impl FerroObjectImpl for Italic {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_current_value(TextElement::font_style_property(), FontStyle::Italic);
    }
}

impl Italic {
    fn static_constructor() {
        TextElement::font_style_property().override_default_value::<Italic>(FontStyle::Italic);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Span::construct() }
    }

    /// Initializes a new instance of the `Italic` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
