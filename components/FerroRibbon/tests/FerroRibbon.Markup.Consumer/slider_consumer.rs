//! Independent consumer subclass dispatches the public RibbonInputBase virtual.
use ferroribbon::controls::{RibbonControlBaseImpl, RibbonInputBaseImpl, RibbonSlider};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, primitives::TemplatedControlImpl};
#[repr(C)]
pub struct ConsumerSlider {
    base: RibbonSlider,
}
ferro_class!(ConsumerSlider:RibbonSlider);
ferro_class_info!(ConsumerSlider{new:ConsumerSlider::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerSlider:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl);
impl ConsumerSlider {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonSlider::construct(),
        })
    }
}
impl RibbonInputBaseImpl for ConsumerSlider {
    fn shows_label_by_default(_c: &Self) -> bool {
        false
    }
}
