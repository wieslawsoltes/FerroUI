use ferroribbon::primitives::RibbonItemContent;
use ferroui_base::{
    FerroObjectImpl, Ref, Size, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_base::{
    input::InputElementImpl,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
};
use ferroui_controls::{ControlImpl, PanelImpl};
use std::cell::Cell;
#[repr(C)]
pub struct ConsumerItemContent {
    base: RibbonItemContent,
    measures: Cell<usize>,
}
ferro_class!(ConsumerItemContent:RibbonItemContent);
ferro_class_info!(ConsumerItemContent {new:ConsumerItemContent::new,markup:{namespace:"FerroRibbon.Consumer",content:Children}});
ferro_impl_classes!(ConsumerItemContent:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl ConsumerItemContent {
    pub fn construct() -> Self {
        Self {
            base: RibbonItemContent::construct(),
            measures: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn measures(&self) -> usize {
        self.measures.get()
    }
}
impl LayoutableImpl for ConsumerItemContent {
    fn measure_override(this: &Self, size: Size) -> Size {
        this.measures.set(this.measures.get() + 1);
        Self::parent_measure_override(this, size)
    }
}
