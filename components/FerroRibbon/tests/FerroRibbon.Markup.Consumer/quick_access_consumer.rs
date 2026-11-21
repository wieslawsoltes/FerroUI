//! A downstream subclass of the public native QAT; same library template/API.
use ferroribbon::controls::RibbonQuickAccessToolBar;
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
pub struct ConsumerQuickAccessToolBar {
    base: RibbonQuickAccessToolBar,
    templates: Cell<usize>,
}
ferro_class!(ConsumerQuickAccessToolBar:RibbonQuickAccessToolBar);
ferro_class_info!(ConsumerQuickAccessToolBar{new:ConsumerQuickAccessToolBar::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerQuickAccessToolBar:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl ConsumerQuickAccessToolBar {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonQuickAccessToolBar::construct(),
            templates: Cell::new(0),
        })
    }
    pub fn templates(&self) -> usize {
        self.templates.get()
    }
}
impl TemplatedControlImpl for ConsumerQuickAccessToolBar {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.templates.set(this.templates.get() + 1);
    }
}
