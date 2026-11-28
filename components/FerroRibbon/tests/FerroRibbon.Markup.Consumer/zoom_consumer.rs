//! Independent subclass and narrowly scoped XAML event callbacks.
use ferroribbon::controls::{RibbonControlBaseImpl, RibbonControlBaseImplExt, RibbonZoomControl};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl, PanelImpl, StackPanel, StackPanelImpl, primitives::TemplatedControlImpl,
};
use std::cell::{Cell, RefCell};

#[repr(C)]
pub struct ConsumerZoom {
    base: RibbonZoomControl,
    invocations: Cell<usize>,
}
ferro_class!(ConsumerZoom:RibbonZoomControl);
ferro_class_info!(ConsumerZoom{new:ConsumerZoom::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerZoom:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl ConsumerZoom {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonZoomControl::construct(),
            invocations: Cell::new(0),
        })
    }
    pub fn invocations(&self) -> usize {
        self.invocations.get()
    }
}
impl RibbonControlBaseImpl for ConsumerZoom {
    fn invoke_core(c: &Self) -> bool {
        c.invocations.set(c.invocations.get() + 1);
        Self::parent_invoke_core(c)
    }
}

#[repr(C)]
pub struct ZoomView {
    base: StackPanel,
    changes: RefCell<Vec<f64>>,
    dialogs: Cell<usize>,
}
ferro_class!(ZoomView:StackPanel);
ferro_class_info!(ZoomView{new:ZoomView::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn Changed(Option<ferroui_base::BoxedValue>,f64)=>ZoomView::changed,fn Dialog(Option<ferroui_base::BoxedValue>,ferroui_base::utilities::EventArgs)=>ZoomView::dialog]}});
ferro_impl_classes!(ZoomView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
impl ZoomView {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            changes: RefCell::new(vec![]),
            dialogs: Cell::new(0),
        })
    }
    pub fn changed(&self, _: Option<ferroui_base::BoxedValue>, value: f64) {
        self.changes.borrow_mut().push(value);
    }
    pub fn dialog(
        &self,
        _: Option<ferroui_base::BoxedValue>,
        _: ferroui_base::utilities::EventArgs,
    ) {
        self.dialogs.set(self.dialogs.get() + 1);
    }
    pub fn reset(&self) {
        self.changes.borrow_mut().clear();
        self.dialogs.set(0);
    }
    pub fn changes(&self) -> Vec<f64> {
        self.changes.borrow().clone()
    }
    pub fn dialogs(&self) -> usize {
        self.dialogs.get()
    }
}
