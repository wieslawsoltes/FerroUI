use super::{InlineImpl, Span, TextElementImpl};
use super::Inline;
use ferroui_base::media::TextDecorations;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl,
};

/// `Underline` element - markup helper for indicating underlined content.
/// Equivalent to a `Span` with the `TextDecorations` property set to
/// underline. Can contain other inline elements.
#[repr(C)]
pub struct Underline {
    base: Span,
}

ferro_class!(Underline: Span);
ferroui_base::ferro_class_info!(Underline { new: Underline::new });
ferro_impl_classes!(Underline: StyledElementImpl, TextElementImpl, InlineImpl);

impl FerroObjectImpl for Underline {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.set_current_value(Inline::text_decorations_property(), Some(TextDecorations::underline()));
    }
}

impl Underline {
    fn static_constructor() {
        Inline::text_decorations_property().override_default_value::<Underline>(Some(TextDecorations::underline()));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Span::construct() }
    }

    /// Initializes a new instance of the `Underline` class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
