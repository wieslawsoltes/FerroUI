use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::TextWrapping;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref,
    StyledElementImpl, Thickness, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControl, ContentControlImpl, ControlImpl, TextBlock};

/// Rich completion/signature content with the framework content-template and
/// presenter contract. Strings wrap; caller-provided controls remain controls.
#[repr(C)]
pub struct CompletionTipContentControl {
    base: ContentControl,
}
ferro_class!(CompletionTipContentControl: ContentControl);
ferroui_base::ferro_class_info!(CompletionTipContentControl {
    new: CompletionTipContentControl::new
});
ferro_impl_classes!(CompletionTipContentControl:
    StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl,
    ControlImpl, TemplatedControlImpl, ContentControlImpl
);
impl FerroObjectImpl for CompletionTipContentControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_padding(Thickness::new(4.0, 2.0, 4.0, 2.0));
        this.set_max_width(640.0);
        TextBlock::set_text_wrapping_on(this, TextWrapping::Wrap);
    }
}
impl CompletionTipContentControl {
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
