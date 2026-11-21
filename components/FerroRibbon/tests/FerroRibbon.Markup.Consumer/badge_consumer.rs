use ferroribbon::controls::RibbonKeyTip;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, primitives::TemplatedControlImpl};
#[repr(C)]
pub struct ConsumerBadge {
    base: RibbonKeyTip,
}
ferro_class!(ConsumerBadge:RibbonKeyTip);
ferro_class_info!(ConsumerBadge {new:ConsumerBadge::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerBadge:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl ConsumerBadge {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonKeyTip::construct(),
        })
    }
}
