// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::automation::peers::HostWindowTitleBarAutomationPeer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    Control, ControlImpl,
};
use std::cell::RefCell;
/// Original current-runtime host title-bar control.
#[repr(C)]
pub struct HostWindowTitleBar {
    base: TemplatedControl,
    background_control: RefCell<Option<Ref<Control>>>,
}
ferro_class!(HostWindowTitleBar:TemplatedControl);
ferro_class_info!(HostWindowTitleBar{new:HostWindowTitleBar::new,markup:{attributes:[TemplatePart("PART_Background",type(Ref<Control>))]}});
ferro_impl_classes!(HostWindowTitleBar:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
impl StyledElementImpl for HostWindowTitleBar {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl ControlImpl for HostWindowTitleBar {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        HostWindowTitleBarAutomationPeer::new(this).upcast()
    }
}
impl TemplatedControlImpl for HostWindowTitleBar {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.background_control
            .replace(e.name_scope().find_as::<Control>("PART_Background"));
    }
}
impl HostWindowTitleBar {
    /// Constructs original state for native inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            background_control: RefCell::new(None),
        }
    }
    /// Creates the original title bar.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn background_control(&self) -> Option<Ref<Control>> {
        self.background_control.borrow().clone()
    }
}
