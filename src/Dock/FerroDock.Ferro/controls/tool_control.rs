// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{automation::peers::ToolControlAutomationPeer, model_projection};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl},
    instantiate,
    interactivity::{InteractiveImpl, RoutingStrategies},
    layout::LayoutableImpl,
    reactive::IDisposable,
    BoxedValue, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::peers::AutomationPeer,
    primitives::{TemplatedControl, TemplatedControlImpl},
    templates::IDataTemplate,
    ControlImpl,
};
use std::{cell::RefCell, rc::Rc};
/// Original native tool host.
#[repr(C)]
pub struct ToolControl {
    base: TemplatedControl,
    pressed: RefCell<Option<Rc<dyn IDisposable>>>,
}
ferro_class!(ToolControl:TemplatedControl);
ferro_class_info!(ToolControl {
    new: ToolControl::new
});
ferro_impl_classes!(ToolControl:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl,FerroObjectImpl);
impl ControlImpl for ToolControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ToolControlAutomationPeer::new(this).upcast()
    }
}
ferro_properties! {impl ToolControl {
    /// Identifies IconTemplate.
    pub fn icon_template_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,Option<BoxedValue>>("IconTemplate",None)}
    /// Identifies HeaderTemplate.
    pub fn header_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("HeaderTemplate",None)}
    /// Identifies ModifiedTemplate.
    pub fn modified_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,Option<Rc<dyn IDataTemplate>>>("ModifiedTemplate",None)}
}}
impl VisualImpl for ToolControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let owner = this.to_ref();
        let pressed = this.add_disposable_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, _| owner.pressed_handler(),
            RoutingStrategies::TUNNEL,
            false,
        );
        this.pressed.replace(Some(pressed));
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        let pressed = this.pressed.borrow().clone();
        if let Some(pressed) = pressed {
            pressed.dispose();
        }
    }
}
impl ToolControl {
    /// Constructs original state for native inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            pressed: RefCell::new(None),
        }
    }
    /// Creates the original native host.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets IconTemplate.
    pub fn icon_template(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_template_property())
    }
    /// Sets IconTemplate.
    pub fn set_icon_template(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_template_property(), value);
    }
    /// Gets HeaderTemplate.
    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }
    /// Sets HeaderTemplate.
    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value);
    }
    /// Gets ModifiedTemplate.
    pub fn modified_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::modified_template_property())
    }
    /// Sets ModifiedTemplate.
    pub fn set_modified_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::modified_template_property(), value);
    }
    fn pressed_handler(&self) {
        if let Some(dock) =
            model_projection::dockable(self.data_context().as_ref()).and_then(|d| d.as_dock())
        {
            if let Some(factory) = dock.factory() {
                if dock.active_dockable().is_some() {
                    if let Some(root) = factory.find_root(
                        Some(
                            dock.active_dockable()
                                .expect("source current active dockable"),
                        ),
                        Some(Rc::new(|_| true)),
                    ) {
                        factory.set_focused_dockable(Some(root), dock.active_dockable());
                    }
                }
            }
        }
    }
}
