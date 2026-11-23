// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::automation::peers::DragPreviewWindowAutomationPeer;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};
/// Original transient window used to display drag-preview content.
#[repr(C)]
pub struct DockDragPreviewWindow {
    base: Window,
}
ferro_class!(DockDragPreviewWindow:Window);
ferro_class_info!(DockDragPreviewWindow {
    new: DockDragPreviewWindow::new
});
ferro_impl_classes!(DockDragPreviewWindow:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl,ContentControlImpl,TopLevelImpl,WindowBaseImpl,WindowImpl);
impl StyledElementImpl for DockDragPreviewWindow {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl ControlImpl for DockDragPreviewWindow {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DragPreviewWindowAutomationPeer::new(this).upcast()
    }
}
impl DockDragPreviewWindow {
    /// Constructs the original native window base for inheritance.
    pub fn construct() -> Self {
        Self {
            base: Window::construct(ferroui_controls::platform::PlatformManager::create_window()),
        }
    }
    /// Creates an unshown native drag-preview window.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
