//! Downstream native subclasses of the actual library controls.
use ferroribbon::controls::{
    IRibbonItem, RibbonControlBaseImpl, RibbonGroup, RibbonItemLayout, RibbonItemsContainer,
    RibbonItemsContainerImpl,
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use std::{cell::Cell, rc::Rc};
#[repr(C)]
pub struct ConsumerRibbonItemsContainer {
    base: RibbonItemsContainer,
}
ferro_class!(ConsumerRibbonItemsContainer:RibbonItemsContainer);
ferro_class_info!(ConsumerRibbonItemsContainer{new:ConsumerRibbonItemsContainer::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerRibbonItemsContainer:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl);
impl ConsumerRibbonItemsContainer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonItemsContainer::construct(),
        })
    }
}
impl RibbonItemsContainerImpl for ConsumerRibbonItemsContainer {
    fn create_container_copy(_this: &Self) -> Ref<RibbonItemsContainer> {
        Self::new().upcast()
    }
    fn get_child_layout(
        _this: &Self,
        _child: Rc<dyn IRibbonItem>,
        mut layout: RibbonItemLayout,
    ) -> RibbonItemLayout {
        layout.size = ferroribbon_core::RibbonItemSize::Medium;
        layout.show_label = Some(true);
        layout
    }
}
#[repr(C)]
pub struct ConsumerRibbonGroup {
    base: RibbonGroup,
    template_count: Cell<usize>,
}
ferro_class!(ConsumerRibbonGroup:RibbonGroup);
ferro_class_info!(ConsumerRibbonGroup{new:ConsumerRibbonGroup::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerRibbonGroup:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl ConsumerRibbonGroup {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonGroup::construct(),
            template_count: Cell::new(0),
        })
    }
    pub fn template_count(&self) -> usize {
        self.template_count.get()
    }
}
impl TemplatedControlImpl for ConsumerRibbonGroup {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.template_count.set(this.template_count.get() + 1);
    }
}
