//! Downstream status-bar inheritance and source-style invocation handlers.
use ferroribbon::controls::{RibbonItemInvokedEventArgs, RibbonStatusBar};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl, PanelImpl, StackPanel, StackPanelImpl,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct ConsumerStatus {
    base: RibbonStatusBar,
    templates: Cell<usize>,
}
ferro_class!(ConsumerStatus:RibbonStatusBar);
ferro_class_info!(ConsumerStatus{new:ConsumerStatus::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerStatus:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl ConsumerStatus {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonStatusBar::construct(),
            templates: Cell::new(0),
        })
    }
    pub fn templates(&self) -> usize {
        self.templates.get()
    }
}
impl TemplatedControlImpl for ConsumerStatus {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        c.templates.set(c.templates.get() + 1);
        Self::parent_on_apply_template(c, e);
    }
}
#[repr(C)]
pub struct StatusView {
    base: StackPanel,
    invoked: RefCell<Vec<Option<String>>>,
}
ferro_class!(StatusView:StackPanel);
ferro_class_info!(StatusView{new:StatusView::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn Invoked(Option<BoxedValue>,Rc<RibbonItemInvokedEventArgs>)=>StatusView::invoked]}});
ferro_impl_classes!(StatusView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
impl StatusView {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            invoked: RefCell::new(vec![]),
        })
    }
    pub fn invoked(&self, _: Option<BoxedValue>, args: Rc<RibbonItemInvokedEventArgs>) {
        self.invoked
            .borrow_mut()
            .push(args.item_id.as_ref().map(|s| s.to_utf8().unwrap()));
    }
    pub fn calls(&self) -> Vec<Option<String>> {
        self.invoked.borrow().clone()
    }
}
