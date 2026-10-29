// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::automation::peers::PinnedDockWindowAutomationPeer;
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
/// Original window used to display floating pinned-dock content.
#[repr(C)]
pub struct PinnedDockWindow {
    base: Window,
}
ferro_class!(PinnedDockWindow:Window);
ferro_class_info!(PinnedDockWindow {
    new: PinnedDockWindow::new
});
ferro_impl_classes!(PinnedDockWindow:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl,ContentControlImpl,TopLevelImpl,WindowBaseImpl,WindowImpl);
impl StyledElementImpl for PinnedDockWindow {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl ControlImpl for PinnedDockWindow {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        PinnedDockWindowAutomationPeer::new(this).upcast()
    }
}
impl PinnedDockWindow {
    /// Constructs the original native window base for inheritance.
    pub fn construct() -> Self {
        Self {
            base: Window::construct(ferroui_controls::platform::PlatformManager::create_window()),
        }
    }
    /// Creates an unshown native pinned-dock window.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
