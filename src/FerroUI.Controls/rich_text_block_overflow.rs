use crate::{ControlImpl, RichTextBlock, RichTextBlockImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    DirectProperty, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, VisualImpl,
    ferro_class, instantiate,
};

/// A continuation of another rich-text view, with its own width and height.
/// Link it with `RichTextBlock::set_overflow_content_target`; the document and
/// its shaped paragraph continuation are shared through weak view references.
#[repr(C)]
pub struct RichTextBlockOverflow {
    base: RichTextBlock,
}
ferro_class!(RichTextBlockOverflow: RichTextBlock);
ferroui_base::ferro_class_info!(RichTextBlockOverflow {
    new: RichTextBlockOverflow::new,
});
ferroui_base::ferro_impl_classes!(RichTextBlockOverflow:
    FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl,
    InputElementImpl, ControlImpl, RichTextBlockImpl);

ferroui_base::ferro_properties! {
    impl RichTextBlockOverflow {
        pub fn content_source_property() -> DirectProperty<RichTextBlockOverflow, Option<Ref<RichTextBlock>>> {
            FerroProperty::register_direct::<RichTextBlockOverflow, _>("ContentSource", Self::content_source, None, None)
        }
    }
}

impl RichTextBlockOverflow {
    /// The primary rich-text view resolved by linked layout setup. Linking a
    /// chain clears this cache; a getter does not force the chain to measure.
    pub fn content_source(&self) -> Option<Ref<RichTextBlock>> {
        self.base.cached_overflow_master()
    }
    pub fn construct() -> Self {
        Self {
            base: RichTextBlock::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
