//! Independent compiled/runtime event consumer; ordinary native control metadata.
use ferroribbon::controls::{
    RibbonContextMenuEventArgs, RibbonCustomizeRequestedEventArgs, RibbonDialogButtonClickEventArgs,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, PanelImpl, StackPanel, StackPanelImpl};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct MenuEventsView {
    base: StackPanel,
    context: RefCell<Option<Rc<RibbonContextMenuEventArgs>>>,
    customization: RefCell<Option<Rc<RibbonCustomizeRequestedEventArgs>>>,
    context_calls: Cell<usize>,
    customize_calls: Cell<usize>,
    dialog_calls: Cell<usize>,
    dialog_cancel: Cell<bool>,
}
ferro_class!(MenuEventsView:StackPanel);
ferro_class_info!(MenuEventsView{new:MenuEventsView::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn OnDialog(Option<BoxedValue>,Rc<RibbonDialogButtonClickEventArgs>)=>MenuEventsView::on_dialog,fn OnContext(Option<BoxedValue>,Rc<RibbonContextMenuEventArgs>)=>MenuEventsView::on_context,fn OnCustomize(Option<BoxedValue>,Rc<RibbonCustomizeRequestedEventArgs>)=>MenuEventsView::on_customize]}});
ferro_impl_classes!(MenuEventsView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
impl MenuEventsView {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            context: RefCell::new(None),
            customization: RefCell::new(None),
            context_calls: Cell::new(0),
            customize_calls: Cell::new(0),
            dialog_calls: Cell::new(0),
            dialog_cancel: Cell::new(true),
        })
    }
    pub fn on_dialog(&self, _: Option<BoxedValue>, args: Rc<RibbonDialogButtonClickEventArgs>) {
        self.dialog_calls.set(self.dialog_calls.get() + 1);
        args.set_cancel(self.dialog_cancel.get());
    }
    pub fn set_dialog_cancel(&self, value: bool) {
        self.dialog_cancel.set(value);
    }
    pub fn dialog_calls(&self) -> usize {
        self.dialog_calls.get()
    }
    pub fn on_context(&self, _: Option<BoxedValue>, args: Rc<RibbonContextMenuEventArgs>) {
        self.context_calls.set(self.context_calls.get() + 1);
        args.handled.set(true);
        self.context.replace(Some(args));
    }
    pub fn on_customize(&self, _: Option<BoxedValue>, args: Rc<RibbonCustomizeRequestedEventArgs>) {
        self.customize_calls.set(self.customize_calls.get() + 1);
        args.handled.set(true);
        self.customization.replace(Some(args));
    }
    pub fn context(&self) -> Option<Rc<RibbonContextMenuEventArgs>> {
        self.context.borrow().clone()
    }
    pub fn customization(&self) -> Option<Rc<RibbonCustomizeRequestedEventArgs>> {
        self.customization.borrow().clone()
    }
    pub fn context_calls(&self) -> usize {
        self.context_calls.get()
    }
    pub fn customize_calls(&self) -> usize {
        self.customize_calls.get()
    }
}
