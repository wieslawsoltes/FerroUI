// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::input::{ICommand, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{
    primitives::{TemplatedControl, TemplatedControlImpl},
    ControlImpl,
};
use std::rc::Rc;
/// The source confirmation shell with title, message and action buttons.
#[repr(C)]
pub struct ConfirmationDialogControl {
    base: TemplatedControl,
}
ferro_class!(ConfirmationDialogControl:TemplatedControl);
ferro_class_info!(ConfirmationDialogControl {
    new: ConfirmationDialogControl::new
});
ferro_impl_classes!(ConfirmationDialogControl:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,VisualImpl);
ferro_properties! {impl ConfirmationDialogControl {
    /// Identifies the source Title styled property.
    pub fn title_property()->StyledProperty<Option<String>> {FerroProperty::register::<ConfirmationDialogControl,_>("Title",None)}
    /// Identifies the source Message styled property.
    pub fn message_property()->StyledProperty<Option<String>> {FerroProperty::register::<ConfirmationDialogControl,_>("Message",None)}
    /// Identifies the source ConfirmText styled property.
    pub fn confirm_text_property()->StyledProperty<Option<String>> {FerroProperty::register::<ConfirmationDialogControl,_>("ConfirmText",None)}
    /// Identifies the source CancelText styled property.
    pub fn cancel_text_property()->StyledProperty<Option<String>> {FerroProperty::register::<ConfirmationDialogControl,_>("CancelText",None)}
    /// Identifies the source ConfirmCommand styled property.
    pub fn confirm_command_property()->StyledProperty<Option<Rc<dyn ICommand>>> {FerroProperty::register::<ConfirmationDialogControl,_>("ConfirmCommand",None)}
    /// Identifies the source CancelCommand styled property.
    pub fn cancel_command_property()->StyledProperty<Option<Rc<dyn ICommand>>> {FerroProperty::register::<ConfirmationDialogControl,_>("CancelCommand",None)}
}}
impl ConfirmationDialogControl {
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
    /// Gets Message.
    pub fn message(&self) -> Option<String> {
        self.get_value(Self::message_property())
    }
    /// Sets Message.
    pub fn set_message(&self, value: Option<String>) {
        self.set_value(Self::message_property(), value);
    }
    /// Gets ConfirmText.
    pub fn confirm_text(&self) -> Option<String> {
        self.get_value(Self::confirm_text_property())
    }
    /// Sets ConfirmText.
    pub fn set_confirm_text(&self, value: Option<String>) {
        self.set_value(Self::confirm_text_property(), value);
    }
    /// Gets CancelText.
    pub fn cancel_text(&self) -> Option<String> {
        self.get_value(Self::cancel_text_property())
    }
    /// Sets CancelText.
    pub fn set_cancel_text(&self, value: Option<String>) {
        self.set_value(Self::cancel_text_property(), value);
    }
    /// Gets ConfirmCommand.
    pub fn confirm_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::confirm_command_property())
    }
    /// Sets ConfirmCommand.
    pub fn set_confirm_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::confirm_command_property(), value);
    }
    /// Gets CancelCommand.
    pub fn cancel_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::cancel_command_property())
    }
    /// Sets CancelCommand.
    pub fn set_cancel_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::cancel_command_property(), value);
    }
}
