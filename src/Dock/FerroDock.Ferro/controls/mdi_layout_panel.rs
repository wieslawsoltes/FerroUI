// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{
    mdi::{ClassicMdiLayoutManager, IMdiLayoutManager, MdiLayoutEntry},
    model_projection,
};
use ferrodock_model::collections::ListHandle;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size,
    StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl};
use std::rc::Rc;
/// Original native panel arranging MDI document windows.
#[repr(C)]
pub struct MdiLayoutPanel {
    base: Panel,
}
ferro_class!(MdiLayoutPanel:Panel);
ferro_class_info!(MdiLayoutPanel {
    new: MdiLayoutPanel::new
});
ferro_impl_classes!(MdiLayoutPanel:StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl FerroObjectImpl for MdiLayoutPanel {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::layout_manager_property().as_property() {
            this.invalidate_arrange();
        }
    }
}
impl LayoutableImpl for MdiLayoutPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let children = this.children();
        let mut index = 0;
        while index < children.count() {
            children.get(index).measure(available);
            index += 1;
        }
        available
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let mut entries = Vec::new();
        let children = this.children();
        let mut index = 0;
        while index < children.count() {
            let child = children.get(index);
            if let Some(document) = model_projection::mdi_document(child.data_context().as_ref()) {
                entries.push(MdiLayoutEntry::new(child, document));
            }
            index += 1;
        }
        let manager = this
            .layout_manager()
            .unwrap_or_else(|| ClassicMdiLayoutManager::instance());
        manager.arrange(&ListHandle::from_plain(entries), final_size);
        final_size
    }
}
ferro_properties! {impl MdiLayoutPanel{
    /// Identifies the original optional layout manager with the shared classic default.
    pub fn layout_manager_property()->StyledProperty<Option<Rc<dyn IMdiLayoutManager>>>{FerroProperty::register::<MdiLayoutPanel,_>("LayoutManager",Some(ClassicMdiLayoutManager::instance() as Rc<dyn IMdiLayoutManager>))}
}}
impl MdiLayoutPanel {
    /// Constructs the original panel base for this or a derived panel.
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
        }
    }
    /// Creates a native MDI layout panel.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the original optional layout manager.
    pub fn layout_manager(&self) -> Option<Rc<dyn IMdiLayoutManager>> {
        self.get_value(Self::layout_manager_property())
    }
    /// Sets the layout manager; None selects the original classic fallback.
    pub fn set_layout_manager(&self, value: Option<Rc<dyn IMdiLayoutManager>>) {
        self.set_value(Self::layout_manager_property(), value);
    }
}
