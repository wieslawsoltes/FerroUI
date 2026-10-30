// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::IDockAdornerWindowFactory;
use crate::automation::peers::DockAdornerWindowAutomationPeer;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    AttachedProperty, FerroObjectImpl, FerroProperty, Ref, StyledElement, StyledElementImpl,
    StyledPropertyOptions, VisualImpl,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};
use std::rc::Rc;
/// Original floating host used for dock-target selectors and indicators.
#[repr(C)]
pub struct DockAdornerWindow {
    base: Window,
}
ferro_class!(DockAdornerWindow:Window);
ferro_class_info!(DockAdornerWindow {
    new: DockAdornerWindow::new
});
ferro_impl_classes!(DockAdornerWindow:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl,ContentControlImpl,TopLevelImpl,WindowBaseImpl,WindowImpl);
impl StyledElementImpl for DockAdornerWindow {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl ControlImpl for DockAdornerWindow {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        DockAdornerWindowAutomationPeer::new(this).upcast()
    }
}
ferro_properties! {impl DockAdornerWindow {
    /// Identifies the inherited optional floating-adornment host factory.
    pub fn factory_property()->AttachedProperty<Option<Rc<dyn IDockAdornerWindowFactory>>>{FerroProperty::register_attached_with::<DockAdornerWindow,StyledElement,_>("Factory",StyledPropertyOptions::new(None).inherits(true))}
}}
impl DockAdornerWindow {
    /// Constructs the original native window base.
    pub fn construct() -> Self {
        Self {
            base: Window::construct(ferroui_controls::platform::PlatformManager::create_window()),
        }
    }
    /// Creates an unshown adorner host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets the factory inherited by a visual scope.
    pub fn get_factory(element: &StyledElement) -> Option<Rc<dyn IDockAdornerWindowFactory>> {
        element.get_value(Self::factory_property())
    }
    /// Sets the optional factory inherited by a visual scope.
    pub fn set_factory(element: &StyledElement, value: Option<Rc<dyn IDockAdornerWindowFactory>>) {
        element.set_value(Self::factory_property(), value);
    }
}
