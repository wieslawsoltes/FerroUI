use ferroribbon::controls::RibbonScreenTip;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use std::cell::Cell;
#[repr(C)]
pub struct ConsumerScreenTip {
    base: RibbonScreenTip,
    applied: Cell<usize>,
}
ferro_class!(ConsumerScreenTip:RibbonScreenTip);
ferro_class_info!(ConsumerScreenTip {new:ConsumerScreenTip::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerScreenTip:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl ConsumerScreenTip {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonScreenTip::construct(),
            applied: Cell::new(0),
        })
    }
    pub fn applications(&self) -> usize {
        self.applied.get()
    }
}
impl TemplatedControlImpl for ConsumerScreenTip {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.applied.set(this.applied.get() + 1);
    }
}
