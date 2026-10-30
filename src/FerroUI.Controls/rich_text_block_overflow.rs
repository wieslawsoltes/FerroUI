use crate::primitives::FlyoutBase;
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
    InputElementImpl, RichTextBlockImpl);

impl ControlImpl for RichTextBlockOverflow {
    /// WinUI's overflow falls back to the context flyout of its primary view, and
    /// shows it for a request only when its selection policy enables it.
    fn effective_context_flyout(this: &Self, for_request: bool) -> Option<Ref<FlyoutBase>> {
        if for_request && !this.context_flyout_is_enabled() {
            return None;
        }
        this.context_flyout().or_else(|| this.inherited_context_flyout())
    }
}

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
