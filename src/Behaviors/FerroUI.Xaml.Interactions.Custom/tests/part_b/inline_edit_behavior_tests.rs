//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/InlineEditBehaviorTests.cs` of
//! Xaml.Behaviors.
//!
//! Adapted: a click of the mouse is a press and a release of the pointer raised on the element,
//! two clicks in a row a double click (`support.rs`).

use super::support::{add_behavior, show_window_with, start_app, MouseTestHelper};
use crate::InlineEditBehavior;
use ferroui_base::input::{InputElement, TappedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutingStrategies};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::Ref;
use ferroui_controls::{Border, StackPanel, TextBox};
use std::rc::Rc;

fn transparent_border() -> Ref<Border> {
    let border = Border::new();
    border.set_width(100.0);
    border.set_height(30.0);
    border.set_background(Some(Brushes::transparent() as Rc<dyn IBrush>));
    border
}

fn text_box() -> Ref<TextBox> {
    let text_box = TextBox::new();
    text_box.set_width(100.0);
    text_box.set_height(30.0);
    text_box
}

#[test]
fn double_tapped_display_control_begins_edit() {
    let _app = start_app();

    let display_control = transparent_border();
    let edit_control = text_box();
    let host = StackPanel::new();
    host.children().add(&display_control);
    host.children().add(&edit_control);
    let behavior = InlineEditBehavior::new();
    behavior.set_display_control(&display_control);
    behavior.set_edit_control(&edit_control);
    add_behavior(&host, &behavior);
    let _window = show_window_with(&host);

    let mouse = MouseTestHelper::new();
    mouse.click(&display_control);
    mouse.click(&display_control);

    assert!(!display_control.is_visible());
    assert!(edit_control.is_visible());
}

#[test]
fn double_tapped_associated_object_begins_edit() {
    let _app = start_app();

    let activation_target = transparent_border();
    let display_control = transparent_border();
    let edit_control = text_box();
    let host = StackPanel::new();
    host.children().add(&activation_target);
    host.children().add(&display_control);
    host.children().add(&edit_control);
    let behavior = InlineEditBehavior::new();
    behavior.set_display_control(&display_control);
    behavior.set_edit_control(&edit_control);
    behavior.set_edit_on_associated_object_double_tapped(true);
    add_behavior(&host, &behavior);
    let _window = show_window_with(&host);

    let mouse = MouseTestHelper::new();
    mouse.click(&activation_target);
    mouse.click(&activation_target);

    assert!(!display_control.is_visible());
    assert!(edit_control.is_visible());
}

#[test]
fn double_tapped_edit_control_does_not_restart_edit() {
    let _app = start_app();

    let activation_target = transparent_border();
    let display_control = transparent_border();
    let edit_control = text_box();
    edit_control.set_text(Some("one two"));
    let host = StackPanel::new();
    host.children().add(&activation_target);
    host.children().add(&display_control);
    host.children().add(&edit_control);
    let behavior = InlineEditBehavior::new();
    behavior.set_display_control(&display_control);
    behavior.set_edit_control(&edit_control);
    behavior.set_edit_on_associated_object_double_tapped(true);
    add_behavior(&host, &behavior);
    let _window = show_window_with(&host);

    let mouse = MouseTestHelper::new();
    mouse.click(&activation_target);
    mouse.click(&activation_target);

    let edit = edit_control.downgrade();
    edit_control.add_handler_with(
        InputElement::double_tapped_event(),
        move |_sender: &Interactive, _e: &TappedEventArgs| {
            if let Some(edit) = edit.upgrade() {
                edit.set_selection_start(1);
                edit.set_selection_end(3);
            }
        },
        RoutingStrategies::BUBBLE,
        false,
    );

    // The clicks on the edit control start a new double click (the reference's clicks on another
    // element do).
    mouse.reset_click_count();
    mouse.click(&edit_control);
    mouse.click(&edit_control);

    assert_eq!(1, edit_control.selection_start());
    assert_eq!(3, edit_control.selection_end());
}
