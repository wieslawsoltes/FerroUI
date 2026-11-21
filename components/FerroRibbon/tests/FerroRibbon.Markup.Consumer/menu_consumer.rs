//! Independent native subclass; inherited menu invocation dispatches the override.
use ferroribbon::compat::{MenuFlyoutItem, MenuFlyoutItemImpl, MenuFlyoutItemImplExt};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl, ItemsControlImpl, MenuItemImpl,
    primitives::{
        HeaderedSelectingItemsControlImpl, SelectingItemsControlImpl, TemplatedControlImpl,
    },
};
use std::cell::Cell;
#[repr(C)]
pub struct ConsumerMenu {
    base: MenuFlyoutItem,
    calls: Cell<usize>,
}
ferro_class!(ConsumerMenu:MenuFlyoutItem);
ferro_class_info!(ConsumerMenu{new:ConsumerMenu::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerMenu:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ItemsControlImpl,SelectingItemsControlImpl,HeaderedSelectingItemsControlImpl,MenuItemImpl);
impl ConsumerMenu {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: MenuFlyoutItem::construct(),
            calls: Cell::new(0),
        })
    }
    pub fn calls(&self) -> usize {
        self.calls.get()
    }
}
impl MenuFlyoutItemImpl for ConsumerMenu {
    fn invoke_menu_core(this: &Self) {
        this.calls.set(this.calls.get() + 1);
        Self::parent_invoke_menu_core(this);
    }
}
