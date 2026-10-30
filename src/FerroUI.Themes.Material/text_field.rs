//! Material decoration slots for an ordinary [`TextBox`].
//!
//! Editing, selection, IME, clipboard, undo and the value automation provider stay
//! owned by FerroUI's text box. These attached properties supply the independent
//! label, affixes, icon slots and assistive row used by its Material template.
//! Anatomy follows AndroidX Material3's `internal/TextFieldImpl.kt` at
//! `e4d933c98df7baa7ea2b83eea6e086915961e0ff` (Apache-2.0).

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    AttachedProperty, BoxedValue, FerroProperty, FerroPropertyChangedEventArgs, ferro_properties,
    ferro_static_type,
};
use ferroui_controls::{DataValidationErrors, TextBox};

/// Attached Material text-field decoration. Works on the existing `TextBox`;
/// choosing a theme does not replace the editor or its automation peer.
///
/// ```xml
/// <TextBox material:MaterialTextField.Label="Amount"
///          material:MaterialTextField.Prefix="$"
///          material:MaterialTextField.Suffix="USD"
///          material:MaterialTextField.SupportingText="Before tax"
///          PlaceholderText="0.00" />
/// ```
pub struct MaterialTextField;
ferro_static_type!(MaterialTextField);

ferro_properties! { impl MaterialTextField {
    /// A label independent of the editor's placeholder. Empty labels are omitted.
    pub fn label_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("Label", None)
    }
    /// Noneditable text before the editable value, with the native 2dp gap.
    pub fn prefix_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("Prefix", None)
    }
    /// Noneditable text after the editable value, with the native 2dp gap.
    pub fn suffix_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("Suffix", None)
    }
    /// Leading content, centered in a minimum 48dp slot. Icons normally measure 24dp.
    /// Use a real button as content when the icon performs an action.
    pub fn leading_icon_property() -> AttachedProperty<Option<BoxedValue>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("LeadingIcon", None)
    }
    /// Trailing content, centered in a minimum 48dp slot. Interactive content retains
    /// its own keyboard and automation contracts.
    pub fn trailing_icon_property() -> AttachedProperty<Option<BoxedValue>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("TrailingIcon", None)
    }
    /// Assistance displayed 4dp below the painted field, outside its container.
    pub fn supporting_text_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("SupportingText", None)
    }
    /// An application validation message. A nonempty message also activates the
    /// error appearance; clearing it restores the supporting text.
    pub fn error_text_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("ErrorText", None)
    }
    /// Activates error appearance without changing FerroUI's validation errors.
    /// Supply `ErrorText` for a specific, localized accessibility description.
    pub fn is_error_property() -> AttachedProperty<bool> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("IsError", false)
    }
    /// Displays the live UTF-16 count, consistent with TextBox positions and MaxLength.
    pub fn show_character_count_property() -> AttachedProperty<bool> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("ShowCharacterCount", false)
    }
    /// Advisory UTF-16 limit. Exceeding it marks the field and counter as errors,
    /// without truncating input or interfering with composition/undo. Zero uses
    /// `TextBox.MaxLength`, or shows an unbounded count when MaxLength is also zero.
    pub fn character_limit_property() -> AttachedProperty<i32> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("CharacterLimit", 0)
    }
    /// Computed message for the template. Applications should set SupportingText
    /// or ErrorText instead of writing this output property.
    pub fn assistive_text_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("AssistiveText", None)
    }
    /// Computed counter for the template; do not set directly.
    pub fn counter_text_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("CounterText", None)
    }
    /// Computed assistance/error/counter description used as the theme's default
    /// AutomationProperties.HelpText. An explicit application value takes priority.
    pub fn automation_description_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<MaterialTextField, TextBox, _>("AutomationDescription", None)
    }
} }

impl MaterialTextField {
    fn static_constructor() {
        for property in [
            Self::label_property().as_property(),
            Self::prefix_property().as_property(),
            Self::suffix_property().as_property(),
            Self::leading_icon_property().as_property(),
            Self::trailing_icon_property().as_property(),
            Self::supporting_text_property().as_property(),
            Self::error_text_property().as_property(),
            Self::is_error_property().as_property(),
            Self::show_character_count_property().as_property(),
            Self::character_limit_property().as_property(),
        ] {
            property
                .changed()
                .add_class_handler::<TextBox>(Self::on_decoration_changed);
        }
        // Class handlers retain no text boxes. Only fields with a counter/limit
        // do count work, and no per-control timers or subscriptions are created.
        TextBox::text_property()
            .changed()
            .add_class_handler::<TextBox>(|field, _| {
                if Self::get_show_character_count(field) || Self::get_character_limit(field) > 0 {
                    Self::refresh(field);
                }
            });
        TextBox::max_length_property()
            .changed()
            .add_class_handler::<TextBox>(|field, _| {
                if Self::get_show_character_count(field) {
                    Self::refresh(field);
                }
            });
        DataValidationErrors::has_errors_property()
            .changed()
            .add_class_handler::<TextBox>(|field, _| {
                if Self::get_supporting_text(field).is_some()
                    || Self::get_error_text(field).is_some()
                    || Self::get_show_character_count(field)
                    || Self::get_is_error(field)
                {
                    Self::refresh(field);
                }
            });
        DataValidationErrors::errors_property()
            .changed()
            .add_class_handler::<TextBox>(|field, _| {
                if Self::get_supporting_text(field).is_some()
                    || Self::get_error_text(field).is_some()
                    || Self::get_show_character_count(field)
                    || Self::get_is_error(field)
                {
                    Self::refresh(field);
                }
            });
    }

    fn on_decoration_changed(field: &TextBox, _change: &FerroPropertyChangedEventArgs<'_>) {
        Self::refresh(field);
    }

    fn refresh(field: &TextBox) {
        let label = present(Self::get_label(field));
        let supporting = present(Self::get_supporting_text(field));
        let error = present(Self::get_error_text(field));
        let show_count = Self::get_show_character_count(field);
        let limit = Self::get_character_limit(field).max(0);
        let limit = if limit > 0 {
            limit
        } else {
            field.max_length().max(0)
        };
        let count = if show_count || limit > 0 {
            field
                .text()
                .as_deref()
                .unwrap_or_default()
                .encode_utf16()
                .count()
        } else {
            0
        };
        let exceeded = limit > 0 && count > limit as usize;
        let is_error = Self::get_is_error(field) || error.is_some() || exceeded;
        let assistive = error.or(supporting);
        let counter = show_count.then(|| {
            if limit > 0 {
                format!("{count} / {limit}")
            } else {
                count.to_string()
            }
        });
        // Preserve the ordinary text-box peer's validation-error priority. A
        // supporting hint must never hide a binding's actual validation error.
        let binding_errors = DataValidationErrors::get_errors(field)
            .map(|errors| {
                errors
                    .iter()
                    .map(|error| ValueTypes::to_display_string(Some(error)))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .and_then(|text| present(Some(text)));
        let mut description = binding_errors
            .or_else(|| assistive.clone())
            .unwrap_or_default();
        if is_error && description.is_empty() {
            // An application-provided ErrorText is preferred for localized UI.
            description.push_str(if exceeded {
                "Character limit exceeded"
            } else {
                "Invalid input"
            });
        }
        if let Some(counter) = &counter {
            if !description.is_empty() {
                description.push_str(". ");
            }
            description.push_str(counter);
        }
        field.set_value(Self::assistive_text_property(), assistive.clone());
        field.set_value(Self::counter_text_property(), counter);
        field.set_value(
            Self::automation_description_property(),
            present(Some(description)),
        );
        let states = field.pseudo_classes();
        states.set(":material-label", label.is_some());
        states.set(":material-support", assistive.is_some());
        states.set(":material-counter", show_count);
        states.set(":material-error", is_error);
        states.set(":material-overlimit", exceeded);
    }

    pub fn get_label(field: &TextBox) -> Option<String> {
        field.get_value(Self::label_property())
    }
    pub fn set_label(field: &TextBox, value: Option<&str>) {
        field.set_value(Self::label_property(), value.map(str::to_owned));
    }
    pub fn get_prefix(field: &TextBox) -> Option<String> {
        field.get_value(Self::prefix_property())
    }
    pub fn set_prefix(field: &TextBox, value: Option<&str>) {
        field.set_value(Self::prefix_property(), value.map(str::to_owned));
    }
    pub fn get_suffix(field: &TextBox) -> Option<String> {
        field.get_value(Self::suffix_property())
    }
    pub fn set_suffix(field: &TextBox, value: Option<&str>) {
        field.set_value(Self::suffix_property(), value.map(str::to_owned));
    }
    pub fn get_leading_icon(field: &TextBox) -> Option<BoxedValue> {
        field.get_value(Self::leading_icon_property())
    }
    pub fn set_leading_icon(field: &TextBox, value: Option<BoxedValue>) {
        field.set_value(Self::leading_icon_property(), value);
    }
    pub fn get_trailing_icon(field: &TextBox) -> Option<BoxedValue> {
        field.get_value(Self::trailing_icon_property())
    }
    pub fn set_trailing_icon(field: &TextBox, value: Option<BoxedValue>) {
        field.set_value(Self::trailing_icon_property(), value);
    }
    pub fn get_supporting_text(field: &TextBox) -> Option<String> {
        field.get_value(Self::supporting_text_property())
    }
    pub fn set_supporting_text(field: &TextBox, value: Option<&str>) {
        field.set_value(Self::supporting_text_property(), value.map(str::to_owned));
    }
    pub fn get_error_text(field: &TextBox) -> Option<String> {
        field.get_value(Self::error_text_property())
    }
    pub fn set_error_text(field: &TextBox, value: Option<&str>) {
        field.set_value(Self::error_text_property(), value.map(str::to_owned));
    }
    pub fn get_is_error(field: &TextBox) -> bool {
        field.get_value(Self::is_error_property())
    }
    pub fn set_is_error(field: &TextBox, value: bool) {
        field.set_value(Self::is_error_property(), value);
    }
    pub fn get_show_character_count(field: &TextBox) -> bool {
        field.get_value(Self::show_character_count_property())
    }
    pub fn set_show_character_count(field: &TextBox, value: bool) {
        field.set_value(Self::show_character_count_property(), value);
    }
    pub fn get_character_limit(field: &TextBox) -> i32 {
        field.get_value(Self::character_limit_property())
    }
    pub fn set_character_limit(field: &TextBox, value: i32) {
        field.set_value(Self::character_limit_property(), value.max(0));
    }
    pub fn get_assistive_text(field: &TextBox) -> Option<String> {
        field.get_value(Self::assistive_text_property())
    }
    pub fn get_counter_text(field: &TextBox) -> Option<String> {
        field.get_value(Self::counter_text_property())
    }
    pub fn get_automation_description(field: &TextBox) -> Option<String> {
        field.get_value(Self::automation_description_property())
    }
}

fn present(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.is_empty())
}
