//! Downstream native toolbar panel subclass; actual inherited layout dispatch.
use ferroribbon::primitives::RibbonToolBarPanel;
use ferroui_base::{
    FerroObjectImpl, Ref, Size, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
};
use ferroui_controls::{ControlImpl, PanelImpl};
use std::cell::Cell;
#[repr(C)]
pub struct ConsumerToolBarPanel {
    base: RibbonToolBarPanel,
    measure_calls: Cell<i32>,
}
ferro_class!(ConsumerToolBarPanel:RibbonToolBarPanel);
ferro_impl_classes!(ConsumerToolBarPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(ConsumerToolBarPanel{new:ConsumerToolBarPanel::new,markup:{namespace:"FerroRibbon.Consumer",content:Children,properties:[MeasureCalls:i32{get:ConsumerToolBarPanel::measure_calls}]}});
impl ConsumerToolBarPanel {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonToolBarPanel::construct(),
            measure_calls: Cell::new(0),
        })
    }
    pub fn measure_calls(&self) -> i32 {
        self.measure_calls.get()
    }
}
impl LayoutableImpl for ConsumerToolBarPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        this.measure_calls.set(this.measure_calls.get() + 1);
        Self::parent_measure_override(this, available)
    }
}

use ferroribbon::controls::RibbonToolBar;
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt,
};
#[repr(C)]
pub struct ConsumerToolBar {
    base: RibbonToolBar,
    applied: Cell<i32>,
}
ferro_class!(ConsumerToolBar:RibbonToolBar);
ferro_impl_classes!(ConsumerToolBar:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(ConsumerToolBar{new:ConsumerToolBar::new,markup:{namespace:"FerroRibbon.Consumer",properties:[Applied:i32{get:ConsumerToolBar::applied}]}});
impl ConsumerToolBar {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonToolBar::construct(),
            applied: Cell::new(0),
        })
    }
    pub fn applied(&self) -> i32 {
        self.applied.get()
    }
}
impl TemplatedControlImpl for ConsumerToolBar {
    fn on_apply_template(t: &Self, e: &TemplateAppliedEventArgs) {
        t.applied.set(t.applied.get() + 1);
        Self::parent_on_apply_template(t, e);
    }
}
