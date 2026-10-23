//! Port of `Views/SettingsView` (markup): three switches of the settings.

use crate::class_prelude::*;
use crate::located_view;

use crate::view_models::SettingsViewModel;
use ferroui_controls::primitives::ToggleButton;
use ferroui_view::prelude::*;
use std::rc::Rc;

located_view!(SettingsView, SettingsViewModel, settings);

fn settings(view_model: &Rc<SettingsViewModel>) -> Element {
    let switch = |label: &str, value: bool, set: fn(&SettingsViewModel, bool)| {
        let view_model = view_model.clone();
        check_box()
            .content(label)
            .is_checked(Some(value))
            .on_changed(ToggleButton::is_checked_property(), move |checked: &Option<bool>| set(&view_model, checked.unwrap_or(false)))
    };
    stack_panel()
        .margin(16.0)
        .spacing(12.0)
        .child(switch("Use dark theme", view_model.use_dark_theme(), SettingsViewModel::set_use_dark_theme))
        .child(switch("Enable animations", view_model.enable_animations(), SettingsViewModel::set_enable_animations))
        .child(switch("Receive release notifications", view_model.receive_release_notifications(), SettingsViewModel::set_receive_release_notifications))
        .into()
}
