//! Downstream inheritance and nullable source-style event signatures.
use ferroribbon::controls::{
    RibbonControlBaseImpl, RibbonControlBaseImplExt, RibbonSegment, RibbonSegmentedControl,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl, PanelImpl, StackPanel, StackPanelImpl, primitives::TemplatedControlImpl,
};
use std::cell::{Cell, RefCell};

#[repr(C)]
pub struct ConsumerSegmented {
    base: RibbonSegmentedControl,
    invokes: Cell<usize>,
}
ferro_class!(ConsumerSegmented:RibbonSegmentedControl);
ferro_class_info!(ConsumerSegmented{new:ConsumerSegmented::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerSegmented:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl RibbonControlBaseImpl for ConsumerSegmented {
    fn invoke_core(this: &Self) -> bool {
        this.invokes.set(this.invokes.get() + 1);
        Self::parent_invoke_core(this)
    }
}
impl ConsumerSegmented {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonSegmentedControl::construct(),
            invokes: Cell::new(0),
        })
    }
    pub fn invokes(&self) -> usize {
        self.invokes.get()
    }
}
#[repr(C)]
pub struct SegmentedView {
    base: StackPanel,
    calls: RefCell<Vec<Option<String>>>,
}
ferro_class!(SegmentedView:StackPanel);
ferro_class_info!(SegmentedView{new:SegmentedView::new,markup:{namespace:"FerroRibbon.Consumer",methods:[fn Selected(Option<BoxedValue>,Option<Ref<RibbonSegment>>)=>SegmentedView::selected]}});
ferro_impl_classes!(SegmentedView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
impl SegmentedView {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: StackPanel::construct(),
            calls: RefCell::new(vec![]),
        })
    }
    pub fn calls(&self) -> Vec<Option<String>> {
        self.calls.borrow().clone()
    }
    pub fn selected(&self, _sender: Option<BoxedValue>, segment: Option<Ref<RibbonSegment>>) {
        self.calls
            .borrow_mut()
            .push(segment.and_then(|s| s.label()));
    }
}
