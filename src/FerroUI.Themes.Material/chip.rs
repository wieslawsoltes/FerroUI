// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: Apache-2.0
//! Material chip decoration on native Button and ToggleButton controls.
//! Geometry and state roles follow AndroidX Chip.kt at
//! e4d933c98df7baa7ea2b83eea6e086915961e0ff; no chip replaces its command or toggle owner.

use ferroui_base::input::{ICommand, InputElementImpl};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    AttachedProperty, BoxedValue, CornerRadius, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties, ferro_routed_event, ferro_static_type, instantiate,
};
use ferroui_controls::platform::{FeedbackAction, PlatformFeedbackExtensions};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Button, ButtonImpl, ContentControlImpl, ControlImpl};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialChipKind {
    #[default]
    Assist,
    Suggestion,
    Filter,
    Input,
}
ferroui_base::ferro_markup_enum!(MaterialChipKind { Assist, Suggestion, Filter, Input }, { namespace: "FerroUI.Themes.Material" });

/// Attached slots and optional dismissal for the Material chip themes.
/// A selectable chip retains the native ToggleButton pattern with a checkbox role;
/// an assist/suggestion chip retains Button's invoke pattern.
///
/// The optional dismiss button has its own 48 dp interactive slot. Like a larger
/// native trailing composable, this increases the chip's painted height to fit it.
/// DismissRequested/Command ask the application to remove its item; the theme never
/// mutates an application's collection or changes the chip's checked state.
pub struct MaterialChip;
ferro_static_type!(MaterialChip);
ferro_properties! { impl MaterialChip {
    pub fn kind_property() -> AttachedProperty<MaterialChipKind> { FerroProperty::register_attached::<MaterialChip, Button, _>("Kind", MaterialChipKind::Assist) }
    pub fn leading_icon_property() -> AttachedProperty<Option<BoxedValue>> { FerroProperty::register_attached::<MaterialChip, Button, _>("LeadingIcon", None) }
    pub fn trailing_icon_property() -> AttachedProperty<Option<BoxedValue>> { FerroProperty::register_attached::<MaterialChip, Button, _>("TrailingIcon", None) }
    /// A circular 24 dp input-chip avatar, taking precedence over its leading icon/check.
    pub fn avatar_property() -> AttachedProperty<Option<BoxedValue>> { FerroProperty::register_attached::<MaterialChip, Button, _>("Avatar", None) }
    /// Shows the standard selected check when no input-chip avatar is supplied.
    pub fn show_selected_icon_property() -> AttachedProperty<bool> { FerroProperty::register_attached::<MaterialChip, Button, _>("ShowSelectedIcon", true) }
    /// Uses the expressive shape overload's compact spacing and symmetric motion roles.
    pub fn is_expressive_property() -> AttachedProperty<bool> { FerroProperty::register_attached::<MaterialChip, Button, _>("IsExpressive", false) }
    pub fn is_dismissible_property() -> AttachedProperty<bool> { FerroProperty::register_attached::<MaterialChip, Button, _>("IsDismissible", false) }
    pub fn dismiss_command_property() -> AttachedProperty<Option<Rc<dyn ICommand>>> { FerroProperty::register_attached::<MaterialChip, Button, _>("DismissCommand", None) }
    pub fn dismiss_command_parameter_property() -> AttachedProperty<Option<BoxedValue>> { FerroProperty::register_attached::<MaterialChip, Button, _>("DismissCommandParameter", None) }
    /// Accessible, localizable name for the independent dismiss action.
    pub fn dismiss_label_property() -> AttachedProperty<String> { FerroProperty::register_attached::<MaterialChip, Button, _>("DismissLabel", "Remove".to_owned()) }
    /// Measured full shape used by expressive selection, including taller custom slots.
    pub fn full_corner_radius_property() -> AttachedProperty<CornerRadius> { FerroProperty::register_attached::<MaterialChip, Button, _>("FullCornerRadius", CornerRadius::uniform(16.0)) }
}}
impl MaterialChip {
    pub fn get_full_corner_radius(button: &Button) -> CornerRadius {
        button.get_value(Self::full_corner_radius_property())
    }
    pub fn get_kind(button: &Button) -> MaterialChipKind {
        button.get_value(Self::kind_property())
    }
    pub fn set_kind(button: &Button, value: MaterialChipKind) {
        button.set_value(Self::kind_property(), value);
    }
    pub fn get_leading_icon(button: &Button) -> Option<BoxedValue> {
        button.get_value(Self::leading_icon_property())
    }
    pub fn set_leading_icon(button: &Button, value: Option<BoxedValue>) {
        button.set_value(Self::leading_icon_property(), value);
    }
    pub fn get_trailing_icon(button: &Button) -> Option<BoxedValue> {
        button.get_value(Self::trailing_icon_property())
    }
    pub fn set_trailing_icon(button: &Button, value: Option<BoxedValue>) {
        button.set_value(Self::trailing_icon_property(), value);
    }
    pub fn get_avatar(button: &Button) -> Option<BoxedValue> {
        button.get_value(Self::avatar_property())
    }
    pub fn set_avatar(button: &Button, value: Option<BoxedValue>) {
        button.set_value(Self::avatar_property(), value);
    }
    pub fn get_show_selected_icon(button: &Button) -> bool {
        button.get_value(Self::show_selected_icon_property())
    }
    pub fn set_show_selected_icon(button: &Button, value: bool) {
        button.set_value(Self::show_selected_icon_property(), value);
    }
    pub fn get_is_expressive(button: &Button) -> bool {
        button.get_value(Self::is_expressive_property())
    }
    pub fn set_is_expressive(button: &Button, value: bool) {
        button.set_value(Self::is_expressive_property(), value);
    }
    pub fn get_is_dismissible(button: &Button) -> bool {
        button.get_value(Self::is_dismissible_property())
    }
    pub fn set_is_dismissible(button: &Button, value: bool) {
        button.set_value(Self::is_dismissible_property(), value);
    }
    pub fn get_dismiss_command(button: &Button) -> Option<Rc<dyn ICommand>> {
        button.get_value(Self::dismiss_command_property())
    }
    pub fn set_dismiss_command(button: &Button, value: Option<Rc<dyn ICommand>>) {
        button.set_value(Self::dismiss_command_property(), value);
    }
    pub fn get_dismiss_command_parameter(button: &Button) -> Option<BoxedValue> {
        button.get_value(Self::dismiss_command_parameter_property())
    }
    pub fn set_dismiss_command_parameter(button: &Button, value: Option<BoxedValue>) {
        button.set_value(Self::dismiss_command_parameter_property(), value);
    }
    pub fn get_dismiss_label(button: &Button) -> String {
        button.get_value(Self::dismiss_label_property())
    }
    pub fn set_dismiss_label(button: &Button, value: &str) {
        button.set_value(Self::dismiss_label_property(), value.to_owned());
    }
    ferro_routed_event!(
        pub fn dismiss_requested_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<MaterialChip, _>("DismissRequested", RoutingStrategies::BUBBLE)
        }
    );
    pub fn dismiss_requested(
        button: &Button,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        button.add_handler(Self::dismiss_requested_event(), handler)
    }
    pub fn request_dismiss(button: &Button) -> bool {
        if !button.is_effectively_enabled() || !Self::get_is_dismissible(button) {
            return false;
        }
        let command = Self::get_dismiss_command(button);
        let parameter = Self::get_dismiss_command_parameter(button);
        if command
            .as_ref()
            .is_some_and(|command| !command.can_execute(parameter.as_ref()))
        {
            return false;
        }
        let event = RoutedEventArgs::with_event_and_source(
            Self::dismiss_requested_event(),
            button.to_ref(),
        );
        button.raise_event(&event);
        if !event.handled() {
            if let Some(command) = command {
                command.execute(parameter.as_ref());
            }
        }
        true
    }
}

/// Template button with a separate invoke/command path; activation never bubbles
/// a chip-body Click and never toggles/removes the body implicitly.
#[repr(C)]
pub struct MaterialChipDismissButton {
    base: Button,
}
ferro_class!(MaterialChipDismissButton: Button);
ferro_class_info!(MaterialChipDismissButton {
    new: MaterialChipDismissButton::new
});
ferro_impl_classes!(MaterialChipDismissButton: FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, LayoutableImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
impl ButtonImpl for MaterialChipDismissButton {
    fn on_click(this: &Self) {
        if !this.is_effectively_enabled() {
            return;
        }
        if let Some(owner) = this
            .templated_parent()
            .and_then(|parent| parent.cast::<Button>())
        {
            if MaterialChip::request_dismiss(&owner) {
                this.perform_feedback(FeedbackAction::click());
            }
        }
    }
}
impl MaterialChipDismissButton {
    pub fn construct() -> Self {
        Self {
            base: Button::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
