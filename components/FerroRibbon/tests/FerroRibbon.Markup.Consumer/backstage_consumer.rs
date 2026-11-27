//! Independent downstream inheritance and inherited template override.
use ferroribbon::controls::{RibbonBackstage, RibbonBackstageItem};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use std::cell::Cell;
#[repr(C)]
pub struct ConsumerBackstage {
    base: RibbonBackstage,
    applies: Cell<i32>,
}
ferro_class!(ConsumerBackstage:RibbonBackstage);
ferro_class_info!(ConsumerBackstage{new:ConsumerBackstage::new,markup:{namespace:"FerroRibbon.Consumer",properties:[ApplyCount:i32{get:ConsumerBackstage::apply_count}]}});
ferro_impl_classes!(ConsumerBackstage:FerroObjectImpl,StyledElementImpl,VisualImpl,InputElementImpl,InteractiveImpl,LayoutableImpl,ControlImpl);
impl ConsumerBackstage {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonBackstage::construct(),
            applies: Cell::new(0),
        })
    }
    pub fn apply_count(&self) -> i32 {
        self.applies.get()
    }
}
impl TemplatedControlImpl for ConsumerBackstage {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.applies.set(this.applies.get() + 1);
        Self::parent_on_apply_template(this, e);
    }
}
#[repr(C)]
pub struct ConsumerBackstageItem {
    base: RibbonBackstageItem,
}
ferro_class!(ConsumerBackstageItem:RibbonBackstageItem);
ferro_class_info!(ConsumerBackstageItem{new:ConsumerBackstageItem::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerBackstageItem:FerroObjectImpl);
impl ConsumerBackstageItem {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonBackstageItem::construct(),
        })
    }
}
