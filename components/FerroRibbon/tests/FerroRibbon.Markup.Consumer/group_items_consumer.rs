//! A downstream subclass of the actual native group-item panel.
use ferroribbon::primitives::RibbonGroupItemsPanel;
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
pub struct ConsumerGroupItemsPanel {
    base: RibbonGroupItemsPanel,
    pub measures: Cell<usize>,
}
ferro_class!(ConsumerGroupItemsPanel:RibbonGroupItemsPanel);
ferro_impl_classes!(ConsumerGroupItemsPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(ConsumerGroupItemsPanel {new:ConsumerGroupItemsPanel::new,markup:{namespace:"FerroRibbon.Consumer"}});
impl ConsumerGroupItemsPanel {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonGroupItemsPanel::construct(),
            measures: Cell::new(0),
        })
    }
}
impl LayoutableImpl for ConsumerGroupItemsPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        this.measures.set(this.measures.get() + 1);
        let size = Self::parent_measure_override(this, available);
        Size::new(size.width + 1., size.height)
    }
}
pub fn register() {
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroribbon_markup_consumer::group_items_consumer",
        "FerroRibbon.Consumer",
    )]);
    ferroui_base::TypeInfo::register_all(&[ConsumerGroupItemsPanel::TYPE]);
}
