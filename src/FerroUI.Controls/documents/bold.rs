use super::{InlineImpl, Span, TextElementImpl};
use super::TextElement;
use ferroui_base::media::FontWeight;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl,
};

/// `Bold` element - markup helper for indicating bolded content. Equivalent
/// to a `Span` with the `FontWeight` property set to bold. Can contain other
/// inline elements.
#[repr(C)]
pub struct Bold {
    base: Span,
}

ferro_class!(Bold: Span);
ferroui_base::ferro_class_info!(Bold { new: Bold::new });
ferro_impl_classes!(Bold: StyledElementImpl, TextElementImpl, InlineImpl);

impl FerroObjectImpl for Bold {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_current_value(TextElement::font_weight_property(), FontWeight::Bold);
    }
}

impl Bold {
    fn static_constructor() {
        TextElement::font_weight_property().override_default_value::<Bold>(FontWeight::Bold);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Span::construct() }
    }

    /// Initializes a new instance of the `Bold` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
