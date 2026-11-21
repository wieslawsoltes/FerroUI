//! Downstream virtual copy factory over the same public native combo implementation.
use ferroribbon::controls::{
    RibbonComboBox, RibbonComboBoxImpl, RibbonControlBaseImpl, RibbonInputBaseImpl,
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, primitives::TemplatedControlImpl};
#[repr(C)]
pub struct ConsumerCombo {
    base: RibbonComboBox,
}
ferro_class!(ConsumerCombo:RibbonComboBox);
ferro_class_info!(ConsumerCombo{new:ConsumerCombo::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerCombo:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl,RibbonInputBaseImpl);
impl ConsumerCombo {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonComboBox::construct(),
        })
    }
}
impl RibbonComboBoxImpl for ConsumerCombo {
    fn create_copy_instance(_this: &Self) -> Ref<RibbonComboBox> {
        Self::new().upcast()
    }
}
