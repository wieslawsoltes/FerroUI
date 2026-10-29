use ferroui_controls::generators::RecycleKey;
// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::ToolPinItemControl;
use crate::automation::peers::ToolPinnedControlAutomationPeer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, Orientation},
    BoxedValue, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{
    automation::peers::AutomationPeer, primitives::TemplatedControlImpl, Control, ControlImpl,
    ItemsControl, ItemsControlImpl, ItemsControlImplExt,
};
/// Original pinned-tool item container with source creation/prepare orientation behavior.
#[repr(C)]
pub struct ToolPinnedControl {
    base: ItemsControl,
}
ferro_class!(ToolPinnedControl:ItemsControl);
ferro_class_info!(ToolPinnedControl {
    new: ToolPinnedControl::new
});
ferro_impl_classes!(ToolPinnedControl:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl);
impl StyledElementImpl for ToolPinnedControl {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl ControlImpl for ToolPinnedControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ToolPinnedControlAutomationPeer::new(this).upcast()
    }
}
impl ItemsControlImpl for ToolPinnedControl {
    fn create_container_for_item_override(
        this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        let item = ToolPinItemControl::new();
        item.set_orientation(this.orientation());
        item.upcast()
    }
    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        Self::parent_prepare_container_for_item_override(this, container, item, index);
        if let Some(pin) = container.cast::<ToolPinItemControl>() {
            pin.set_orientation(this.orientation());
        }
    }
    fn needs_container_override(
        this: &Self,
        item: &Option<BoxedValue>,
        _index: i32,
    ) -> (bool, Option<RecycleKey>) {
        this.needs_container::<ToolPinItemControl>(item)
    }
}
ferro_properties! {impl ToolPinnedControl {
    /// Identifies Orientation.
    pub fn orientation_property()->StyledProperty<Orientation> {FerroProperty::register::<Self,Orientation>("Orientation",Orientation::Vertical)}
}}
impl ToolPinnedControl {
    /// Constructs the native source class for inheritance.
    pub fn construct() -> Self {
        Self {
            base: ItemsControl::construct(),
        }
    }
    /// Creates the source pinned-tools container.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets Orientation.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    /// Sets Orientation without inventing live container propagation absent from the source.
    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value);
    }
}
