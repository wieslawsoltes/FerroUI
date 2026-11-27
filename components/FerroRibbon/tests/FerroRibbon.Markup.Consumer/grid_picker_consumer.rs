//! Independent native subclass and a handler with the original value-type event signature.
use ferroribbon::controls::RibbonGridPicker;
use ferroribbon_core::model::RibbonGridSize;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl, PanelImpl, StackPanel, StackPanelImpl, primitives::TemplatedControlImpl,
};
use std::cell::RefCell;
#[repr(C)]
pub struct ConsumerGridPicker {
    base: RibbonGridPicker,
}
ferro_class!(ConsumerGridPicker:RibbonGridPicker);
ferro_class_info!(ConsumerGridPicker{new:ConsumerGridPicker::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerGridPicker:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl ConsumerGridPicker {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonGridPicker::construct(),
        })
    }
}
#[repr(C)]
pub struct GridPickView {
    base: StackPanel,
    calls: RefCell<Vec<RibbonGridSize>>,
}
ferro_class!(GridPickView:StackPanel);
ferro_class_info!(GridPickView{new:GridPickView::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn Picked(Option<BoxedValue>,RibbonGridSize)=>GridPickView::picked]}});
ferro_impl_classes!(GridPickView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
impl GridPickView {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            calls: RefCell::new(vec![]),
        })
    }
    pub fn calls(&self) -> Vec<RibbonGridSize> {
        self.calls.borrow().clone()
    }
    pub fn picked(&self, _: Option<BoxedValue>, size: RibbonGridSize) {
        self.calls.borrow_mut().push(size);
    }
}
