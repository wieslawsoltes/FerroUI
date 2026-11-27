//! Downstream native subclass exercises inherited Gallery virtual dispatch.
use ferroribbon::controls::{
    RibbonControlBaseImpl, RibbonControlBaseImplExt, RibbonGallery, RibbonItemLayout,
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, primitives::TemplatedControlImpl};
use std::cell::Cell;
#[repr(C)]
pub struct ConsumerGallery {
    base: RibbonGallery,
    layout_calls: Cell<i32>,
}
ferro_class!(ConsumerGallery:RibbonGallery);
ferro_impl_classes!(ConsumerGallery:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_class_info!(ConsumerGallery{new:ConsumerGallery::new,markup:{namespace:"FerroRibbon.Consumer",properties:[LayoutCalls:i32{get:ConsumerGallery::layout_calls}]}});
impl ConsumerGallery {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonGallery::construct(),
            layout_calls: Cell::new(0),
        })
    }
    pub fn layout_calls(&self) -> i32 {
        self.layout_calls.get()
    }
}
impl RibbonControlBaseImpl for ConsumerGallery {
    fn on_layout_applied_core(this: &Self, layout: RibbonItemLayout) {
        Self::parent_on_layout_applied_core(this, layout);
        this.layout_calls.set(this.layout_calls.get() + 1);
    }
}
