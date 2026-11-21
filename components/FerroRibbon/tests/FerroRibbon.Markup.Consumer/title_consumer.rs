//! Downstream title-bar inheritance and source-style invocation handlers.
use ferroribbon::controls::{RibbonHandledEventArgs, RibbonTitleBar};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl, PanelImpl, StackPanel, StackPanelImpl,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use std::{cell::Cell, rc::Rc};
#[repr(C)]
pub struct ConsumerTitle {
    base: RibbonTitleBar,
    templates: Cell<usize>,
}
ferro_class!(ConsumerTitle:RibbonTitleBar);
ferro_class_info!(ConsumerTitle{new:ConsumerTitle::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerTitle:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl ConsumerTitle {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonTitleBar::construct(),
            templates: Cell::new(0),
        })
    }
    pub fn templates(&self) -> usize {
        self.templates.get()
    }
}
impl TemplatedControlImpl for ConsumerTitle {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        c.templates.set(c.templates.get() + 1);
        Self::parent_on_apply_template(c, e);
    }
}
#[repr(C)]
pub struct TitleView {
    base: StackPanel,
    invoked: Cell<usize>,
}
ferro_class!(TitleView:StackPanel);
ferro_class_info!(TitleView{new:TitleView::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn Invoked(Option<BoxedValue>,Rc<RibbonHandledEventArgs>)=>TitleView::invoked]}});
ferro_impl_classes!(TitleView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
impl TitleView {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            invoked: Cell::new(0),
        })
    }
    pub fn invoked(&self, _: Option<BoxedValue>, args: Rc<RibbonHandledEventArgs>) {
        self.invoked.set(self.invoked.get() + 1);
        args.set_handled(true);
    }
    pub fn calls(&self) -> usize {
        self.invoked.get()
    }
}
