//! Arranged anatomy, editor and automation regressions for Material text fields.

use super::support::start_themed_application;
use crate::{MaterialTextField, MaterialTextFieldBorder, MaterialTextFieldLayout};
use ferroui_base::input::{KeyModifiers, KeyboardDevice, NavigationMethod};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::FlowDirection;
use ferroui_base::{ObjectType, Ref, Visual};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::presenters::{ContentPresenter, TextPresenter};
use ferroui_controls::{
    Border, Button, Control, DataValidationErrors, Grid, ScrollViewer, TextBlock, TextBox, Window,
};
use std::rc::Rc;

fn host(field: &TextBox) -> Ref<Window> {
    let frame = Border::new();
    frame.set_horizontal_alignment(HorizontalAlignment::Left);
    frame.set_vertical_alignment(VerticalAlignment::Top);
    frame.set_child(field.to_ref());
    let window = Window::new();
    window.set_width(520.0);
    window.set_height(360.0);
    window.set_content(Some(Control::boxed(&frame)));
    window.show();
    window.update_layout();
    clear_focus();
    window.update_layout();
    window
}

fn clear_focus() {
    KeyboardDevice::instance().unwrap().set_focused_element(
        None,
        NavigationMethod::Unspecified,
        KeyModifiers::empty(),
    );
}

fn named<T: ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|child| child.name().as_deref() == Some(name))
        .find_map(|child| child.cast::<T>())
        .unwrap_or_else(|| panic!("missing Material text-field part {name}"))
}

fn icon() -> Ref<Border> {
    let icon = Border::new();
    icon.set_width(24.0);
    icon.set_height(24.0);
    icon
}

#[test]
fn independent_label_placeholder_and_affixes_follow_the_input_phase() {
    let _app = start_themed_application();
    let field = TextBox::new();
    MaterialTextField::set_label(&field, Some("Amount"));
    MaterialTextField::set_prefix(&field, Some("$"));
    MaterialTextField::set_suffix(&field, Some("USD"));
    field.set_placeholder_text(Some("0.00"));
    let window = host(&field);
    let label: Ref<TextBlock> = named(&field, "MaterialFloatingLabel");
    let placeholder: Ref<TextBlock> = named(&field, "PlaceholderTextContentPresenter");
    let prefix: Ref<TextBlock> = named(&field, "MaterialPrefix");
    let suffix: Ref<TextBlock> = named(&field, "MaterialSuffix");
    let panel: Ref<MaterialTextFieldLayout> = named(&field, "MaterialFieldLayout");
    assert_eq!(Some("Amount".to_owned()), label.text());
    assert_eq!(Some("0.00".to_owned()), placeholder.text());
    assert!(label.is_visible());
    assert!(!placeholder.is_visible(), "focus={}, empty={}", field.is_focused(), field.classes().contains(":empty"));
    assert!(!prefix.is_visible());
    assert!(!suffix.is_visible());
    assert_eq!(0.0, panel.label_progress());

    assert!(field.focus());
    window.update_layout();
    assert!(placeholder.is_visible());
    assert!(prefix.is_visible());
    assert!(suffix.is_visible());
    assert_eq!(1.0, panel.label_progress());
    assert_eq!(12.0, label.font_size());
    assert_eq!(16.0, label.line_height());
    assert_eq!(8.0, label.bounds().y);

    field.set_text(Some("15"));
    clear_focus();
    window.update_layout();
    assert!(!placeholder.is_visible());
    assert!(prefix.is_visible());
    assert_eq!(1.0, panel.label_progress());
    let editor: Ref<TextPresenter> = named(&field, "PART_TextPresenter");
    assert_eq!(Some("15".to_owned()), editor.text());
}

#[test]
fn label_and_affix_slots_do_not_replace_text_selection_or_ime_composition() {
    let _app = start_themed_application();
    let field = TextBox::new();
    MaterialTextField::set_label(&field, Some("Name"));
    MaterialTextField::set_prefix(&field, Some("@"));
    MaterialTextField::set_show_character_count(&field, true);
    let window = host(&field);
    let editor: Ref<TextPresenter> = named(&field, "PART_TextPresenter");
    editor.set_preedit_text(Some("か"));
    window.update_layout();
    assert_eq!(None, field.text());
    assert_eq!(
        Some("0".to_owned()),
        MaterialTextField::get_counter_text(&field)
    );
    let panel: Ref<MaterialTextFieldLayout> = named(&field, "MaterialFieldLayout");
    assert_eq!(1.0, panel.label_progress());
    editor.set_preedit_text(None);
    window.update_layout();
    assert!(field.classes().contains(":empty"));
    assert_eq!(0.0, panel.label_progress(), "focus={}", field.is_focused());
    field.set_text(Some("Material"));
    field.set_selection_start(1);
    field.set_selection_end(4);
    assert_eq!(Some("Material".to_owned()), editor.text());
    assert_eq!(1, editor.selection_start());
    assert_eq!(4, editor.selection_end());
}

#[test]
fn icons_use_native_slots_and_affixes_have_exact_two_dp_editor_gaps() {
    let _app = start_themed_application();
    let field = TextBox::new();
    MaterialTextField::set_leading_icon(&field, Some(Control::boxed(&icon())));
    MaterialTextField::set_trailing_icon(&field, Some(Control::boxed(&icon())));
    MaterialTextField::set_prefix(&field, Some("$"));
    MaterialTextField::set_suffix(&field, Some("USD"));
    field.set_text(Some("15"));
    let _window = host(&field);
    let leading: Ref<ContentPresenter> = named(&field, "MaterialLeadingIcon");
    let trailing: Ref<ContentPresenter> = named(&field, "MaterialTrailingIcon");
    let prefix: Ref<TextBlock> = named(&field, "MaterialPrefix");
    let suffix: Ref<TextBlock> = named(&field, "MaterialSuffix");
    let editor: Ref<ScrollViewer> = named(&field, "PART_ScrollViewer");
    assert_eq!(48.0, leading.bounds().width);
    assert_eq!(48.0, leading.bounds().height);
    assert_eq!(48.0, trailing.bounds().width);
    assert_eq!(52.0, prefix.bounds().x);
    assert_eq!(prefix.bounds().right() + 2.0, editor.bounds().x);
    assert_eq!(editor.bounds().right() + 2.0, suffix.bounds().x);
    assert_eq!(trailing.bounds().x - 4.0, suffix.bounds().right());
}

#[test]
fn outlined_label_moves_from_icon_inset_to_outline_and_reserves_its_top_space() {
    let _app = start_themed_application();
    let field = TextBox::new();
    field.classes().add("outlined");
    MaterialTextField::set_label(&field, Some("Account"));
    MaterialTextField::set_leading_icon(&field, Some(Control::boxed(&icon())));
    let window = host(&field);
    let label: Ref<TextBlock> = named(&field, "MaterialFloatingLabel");
    let surface: Ref<MaterialTextFieldBorder> = named(&field, "BorderElement");
    assert_eq!(52.0, label.bounds().x);
    assert_eq!(8.0, surface.bounds().y);
    assert_eq!(56.0, surface.bounds().height);
    assert_eq!(64.0, field.bounds().height);
    assert_eq!(0.0, surface.notch_open());
    field.set_text(Some("value"));
    window.update_layout();
    assert_eq!(16.0, label.bounds().x);
    assert_eq!(surface.bounds().y - 8.0, label.bounds().y);
    assert_eq!(label.bounds().x, surface.notch_start());
    assert_eq!(label.bounds().width, surface.notch_width());
    assert_eq!(1.0, surface.notch_open());
}

#[test]
fn label_progress_interpolates_position_and_typography_from_one_spring_value() {
    let _app = start_themed_application();
    let field = TextBox::new();
    field.classes().add("outlined");
    MaterialTextField::set_label(&field, Some("Account"));
    MaterialTextField::set_leading_icon(&field, Some(Control::boxed(&icon())));
    let window = host(&field);
    let panel: Ref<MaterialTextFieldLayout> = named(&field, "MaterialFieldLayout");
    let label: Ref<TextBlock> = named(&field, "MaterialFloatingLabel");
    panel.set_label_progress(0.5);
    window.update_layout();
    assert_eq!(34.0, label.bounds().x);
    assert_eq!(14.0, label.font_size());
    assert_eq!(20.0, label.line_height());
    let surface: Ref<MaterialTextFieldBorder> = named(&field, "BorderElement");
    assert_eq!(0.5, surface.notch_open());
    assert_eq!(
        8.0 + (56.0 - label.bounds().height) / 4.0 - label.bounds().height / 4.0,
        label.bounds().y
    );
}

#[test]
fn supporting_text_and_counter_are_outside_the_56dp_painted_container() {
    let _app = start_themed_application();
    let field = TextBox::new();
    field.set_text(Some("12"));
    MaterialTextField::set_supporting_text(&field, Some("Before tax"));
    MaterialTextField::set_show_character_count(&field, true);
    MaterialTextField::set_character_limit(&field, 10);
    let window = host(&field);
    let surface: Ref<MaterialTextFieldBorder> = named(&field, "BorderElement");
    let supporting: Ref<Grid> = named(&field, "MaterialSupportingRow");
    let message: Ref<TextBlock> = named(&field, "MaterialSupportingText");
    let counter: Ref<TextBlock> = named(&field, "MaterialCharacterCounter");
    assert_eq!(56.0, surface.bounds().height);
    assert_eq!(surface.bounds().bottom() + 4.0, supporting.bounds().y);
    assert_eq!(16.0, supporting.bounds().x);
    assert_eq!(Some("Before tax".to_owned()), message.text());
    assert_eq!(Some("2 / 10".to_owned()), counter.text());
    assert!(field.bounds().height >= 76.0);
    MaterialTextField::set_supporting_text(&field, None);
    MaterialTextField::set_show_character_count(&field, false);
    window.update_layout();
    assert!(!supporting.is_visible());
    assert_eq!(56.0, field.bounds().height);
}

#[test]
fn advisory_counter_uses_utf16_and_never_truncates_or_replaces_validation_state() {
    let _app = start_themed_application();
    let field = TextBox::new();
    MaterialTextField::set_show_character_count(&field, true);
    MaterialTextField::set_character_limit(&field, 4);
    field.set_text(Some("A😀e\u{301}"));
    let window = host(&field);
    assert_eq!(
        Some("5 / 4".to_owned()),
        MaterialTextField::get_counter_text(&field)
    );
    assert_eq!(Some("A😀e\u{301}".to_owned()), field.text());
    assert!(field.classes().contains(":material-overlimit"));
    assert!(field.classes().contains(":material-error"));
    assert!(!DataValidationErrors::get_has_errors(&field));
    MaterialTextField::set_character_limit(&field, 5);
    window.update_layout();
    assert!(!field.classes().contains(":material-overlimit"));
    assert!(!field.classes().contains(":material-error"));
    field.set_text(Some("A"));
    assert_eq!(
        Some("1 / 5".to_owned()),
        MaterialTextField::get_counter_text(&field)
    );
}

#[test]
fn field_keeps_edit_automation_with_live_label_help_and_application_overrides() {
    let _app = start_themed_application();
    let field = TextBox::new();
    MaterialTextField::set_label(&field, Some("Account"));
    MaterialTextField::set_supporting_text(&field, Some("Use your work address"));
    let window = host(&field);
    let peer = ControlAutomationPeer::create_peer_for_element(&field);
    assert_eq!(
        AutomationControlType::Edit,
        peer.get_automation_control_type()
    );
    assert_eq!("Account", peer.get_name());
    assert_eq!("Use your work address", peer.get_help_text());
    MaterialTextField::set_error_text(&field, Some("Address required"));
    window.update_layout();
    assert_eq!("Address required", peer.get_help_text());
    assert!(field.classes().contains(":material-error"));
    MaterialTextField::set_error_text(&field, None);
    window.update_layout();
    assert_eq!("Use your work address", peer.get_help_text());
    AutomationProperties::set_name(&field, Some("Custom field name"));
    AutomationProperties::set_help_text(&field, Some("Custom assistance"));
    MaterialTextField::set_label(&field, Some("Changed"));
    MaterialTextField::set_error_text(&field, Some("Changed error"));
    window.update_layout();
    assert_eq!("Custom field name", peer.get_name());
    assert_eq!("Custom assistance", peer.get_help_text());
}

#[test]
fn binding_validation_messages_take_precedence_over_supporting_hint_in_automation() {
    let _app = start_themed_application();
    let field = TextBox::new();
    MaterialTextField::set_supporting_text(&field, Some("Choose a value"));
    let window = host(&field);
    let peer = ControlAutomationPeer::create_peer_for_element(&field);
    DataValidationErrors::set_errors(&field, Some(vec![Rc::new("Binding error".to_owned())]));
    window.update_layout();
    assert_eq!("Binding error", peer.get_help_text());
    DataValidationErrors::set_errors(&field, Some(vec![Rc::new("Updated error".to_owned())]));
    window.update_layout();
    assert_eq!("Updated error", peer.get_help_text());
    DataValidationErrors::clear_errors(&field);
    window.update_layout();
    assert_eq!("Choose a value", peer.get_help_text());
}

#[test]
fn disabled_error_field_uses_disabled_roles_and_rtl_keeps_logical_slot_geometry() {
    let _app = start_themed_application();
    let field = TextBox::new();
    field.classes().add("outlined");
    field.set_flow_direction(FlowDirection::RightToLeft);
    MaterialTextField::set_label(&field, Some("Value"));
    MaterialTextField::set_leading_icon(&field, Some(Control::boxed(&icon())));
    MaterialTextField::set_error_text(&field, Some("Invalid value"));
    field.set_text(Some("value"));
    field.set_is_enabled(false);
    let _window = host(&field);
    let label: Ref<TextBlock> = named(&field, "MaterialFloatingLabel");
    let leading: Ref<ContentPresenter> = named(&field, "MaterialLeadingIcon");
    let surface: Ref<MaterialTextFieldBorder> = named(&field, "BorderElement");
    assert_eq!(FlowDirection::RightToLeft, leading.flow_direction());
    assert_eq!(0.0, leading.bounds().x);
    assert_eq!(16.0, label.bounds().x);
    assert_eq!(0.38, label.foreground().unwrap().opacity());
    assert_eq!(0.38, surface.border_brush().unwrap().opacity());
    assert_eq!(1.0, surface.border_thickness().top);
}

#[test]
fn material_fields_opt_into_native_minimum_width_and_clear_button() {
    let _app = start_themed_application();
    let field = TextBox::new();
    field.set_text(Some("value"));
    let window = host(&field);
    assert!(field.focus());
    let clear: Ref<Button> = named(&field, "DeleteButton");
    assert_eq!(120.0, field.min_width());
    assert!(!clear.is_visible());
    field.classes().add("nativeMinWidth");
    field.classes().add("clearButton");
    window.update_layout();
    assert_eq!(280.0, field.min_width());
    assert!(clear.is_visible());
    clear.command().unwrap().execute(None);
    assert!(field.text().is_none_or(|text| text.is_empty()));
}
