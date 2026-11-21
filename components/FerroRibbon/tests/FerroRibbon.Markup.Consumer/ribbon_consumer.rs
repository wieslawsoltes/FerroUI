//! Independent downstream inheritance/template dispatch of the actual root.
use ferroribbon::controls::Ribbon;
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
pub struct ConsumerRibbon {
    base: Ribbon,
    templates: Cell<usize>,
}
ferro_class!(ConsumerRibbon:Ribbon);
ferro_class_info!(ConsumerRibbon{new:ConsumerRibbon::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerRibbon:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,LayoutableImpl,ControlImpl);
impl ConsumerRibbon {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Ribbon::construct(),
            templates: Cell::new(0),
        })
    }
    pub fn templates(&self) -> usize {
        self.templates.get()
    }
}
impl TemplatedControlImpl for ConsumerRibbon {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.templates.set(this.templates.get() + 1);
    }
}
