//! Independent subclass of the library's actual split control.
use ferroribbon::controls::{
    RibbonControlBaseImpl, RibbonSplitButton, RibbonSplitButtonImpl, RibbonSplitButtonImplExt,
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, primitives::TemplatedControlImpl};
use std::cell::Cell;
#[repr(C)]
pub struct ConsumerSplit {
    base: RibbonSplitButton,
    calls: Cell<usize>,
}
ferro_class!(ConsumerSplit:RibbonSplitButton);
ferro_class_info!(ConsumerSplit{new:ConsumerSplit::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerSplit:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl);
impl ConsumerSplit {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonSplitButton::construct(),
            calls: Cell::new(0),
        })
    }
    pub fn calls(&self) -> usize {
        self.calls.get()
    }
}
impl RibbonSplitButtonImpl for ConsumerSplit {
    fn invoke_primary(this: &Self) {
        this.calls.set(this.calls.get() + 1);
        Self::parent_invoke_primary(this);
    }
}
