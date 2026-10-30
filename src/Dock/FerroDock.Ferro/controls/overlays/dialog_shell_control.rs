// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::input::{ICommand, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::BoxedValue;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{
    primitives::{TemplatedControl, TemplatedControlImpl},
    ControlImpl,
};
use std::rc::Rc;
/// The source titled dialog container with a close affordance.
#[repr(C)]
pub struct DialogShellControl {
    base: TemplatedControl,
}
ferro_class!(DialogShellControl:TemplatedControl);
ferro_class_info!(DialogShellControl {new:DialogShellControl::new,markup:{content:Content,property_attributes:[Content:[DependsOn("ContentTemplate")]]}});
ferro_impl_classes!(DialogShellControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,VisualImpl);
ferro_properties! {impl DialogShellControl {
    /// Identifies the source Title styled property.
    pub fn title_property()->StyledProperty<Option<String>> {FerroProperty::register::<DialogShellControl,_>("Title",None)}
    /// Identifies the source Content styled property.
    pub fn content_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<DialogShellControl,_>("Content",None)}
    /// Identifies the source ContentTemplate styled property.
    pub fn content_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<DialogShellControl,_>("ContentTemplate",None)}
    /// Identifies the source CloseCommand styled property.
    pub fn close_command_property()->StyledProperty<Option<Rc<dyn ICommand>>> {FerroProperty::register::<DialogShellControl,_>("CloseCommand",None)}
    /// Identifies the source IsCloseVisible styled property.
    pub fn is_close_visible_property()->StyledProperty<bool> {FerroProperty::register::<DialogShellControl,_>("IsCloseVisible",true)}
}}
impl DialogShellControl {
    /// Constructs the native base and source property defaults.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
        }
    }
    /// Creates the source dialog shell.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets Title.
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }
    /// Sets Title.
    pub fn set_title(&self, value: Option<String>) {
        self.set_value(Self::title_property(), value);
    }
    /// Gets Content.
    pub fn content(&self) -> Option<BoxedValue> {
        self.get_value(Self::content_property())
    }
    /// Sets Content.
    pub fn set_content(&self, value: Option<BoxedValue>) {
        self.set_value(Self::content_property(), value);
    }
    /// Gets ContentTemplate.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::content_template_property())
    }
    /// Sets ContentTemplate.
    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::content_template_property(), value);
    }
    /// Gets CloseCommand.
    pub fn close_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::close_command_property())
    }
    /// Sets CloseCommand.
    pub fn set_close_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::close_command_property(), value);
    }
    /// Gets IsCloseVisible.
    pub fn is_close_visible(&self) -> bool {
        self.get_value(Self::is_close_visible_property())
    }
    /// Sets IsCloseVisible.
    pub fn set_is_close_visible(&self, value: bool) {
        self.set_value(Self::is_close_visible_property(), value);
    }
}
