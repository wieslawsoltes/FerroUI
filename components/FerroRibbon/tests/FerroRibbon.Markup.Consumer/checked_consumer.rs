//! Independent downstream class: inherited item interface must dispatch override.
use ferroribbon::controls::{
    RibbonToggleButton, RibbonToggleButtonImpl, RibbonToggleButtonImplExt,
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ButtonImpl, ContentControlImpl, ControlImpl,
    primitives::{TemplatedControlImpl, ToggleButtonImpl},
};
use std::cell::Cell;
#[repr(C)]
pub struct ConsumerToggle {
    base: RibbonToggleButton,
    calls: Cell<usize>,
}
ferro_class!(ConsumerToggle:RibbonToggleButton);
ferro_class_info!(ConsumerToggle{new:ConsumerToggle::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerToggle:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,LayoutableImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,ToggleButtonImpl);
impl ConsumerToggle {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonToggleButton::construct(),
            calls: Cell::new(0),
        })
    }
    pub fn calls(&self) -> usize {
        self.calls.get()
    }
}
impl RibbonToggleButtonImpl for ConsumerToggle {
    fn invoke_core(this: &Self) -> bool {
        this.calls.set(this.calls.get() + 1);
        Self::parent_invoke_core(this)
    }
}
