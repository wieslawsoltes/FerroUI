//! The descriptions of objects in diagnostics (the debug display helper of
//! the reference and the base levels of its `BuildDebugDisplay`).
//!
//! `BuildDebugDisplay` is an internal virtual of the root class of the
//! object model in the reference. The object model has no such virtual
//! here, so the classes of this crate that extend the description have a
//! `build_debug_display` method of their own and [`build_debug_display`]
//! dispatches to them for an object of an unknown class.

use crate::presenters::ContentPresenter;
use crate::primitives::HeaderedContentControl;
use crate::{ContentControl, NativeMenuItem, TableViewColumn, TableViewColumnHeader, TextBlock};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{BoxedValue, FerroObject, StyledElement};

/// Appends the description of an object: the one of its class where the
/// class extends it, the base description otherwise.
pub(crate) fn build_debug_display(object: &FerroObject, builder: &mut String, include_content: bool) {
    if let Some(item) = object.downcast_ref::<NativeMenuItem>() {
        item.build_debug_display(builder, include_content);
    } else if let Some(column) = object.downcast_ref::<TableViewColumn>() {
        column.build_debug_display(builder, include_content);
    } else if let Some(header) = object.downcast_ref::<TableViewColumnHeader>() {
        header.build_debug_display(builder, include_content);
    } else if let Some(control) = object.downcast_ref::<HeaderedContentControl>() {
        // A derived class comes before its base class: the first match describes the object.
        control.build_debug_display(builder, include_content);
    } else if let Some(control) = object.downcast_ref::<ContentControl>() {
        control.build_debug_display(builder, include_content);
    } else if let Some(presenter) = object.downcast_ref::<ContentPresenter>() {
        presenter.build_debug_display(builder, include_content);
    } else if let Some(text_block) = object.downcast_ref::<TextBlock>() {
        text_block.build_debug_display(builder, include_content);
    } else {
        build_base_debug_display(object, builder);
    }
}

/// Appends the description the base classes give: the name of the type
/// and, for a styled element, its name.
pub(crate) fn build_base_debug_display(object: &FerroObject, builder: &mut String) {
    builder.push_str(&debug_type_name(object));

    if let Some(element) = object.downcast_ref::<StyledElement>() {
        append_optional_value(builder, "Name", element.name().as_deref());
    }
}

/// The name of the type of an object in a description: the simple name
/// for the types of the framework, the namespace-qualified name for any
/// other type.
pub(crate) fn debug_type_name(object: &FerroObject) -> String {
    let type_ = object.get_type();
    let namespace = type_.namespace();

    if namespace == "FerroUI" || namespace.starts_with("FerroUI.") {
        type_.name().to_string()
    } else {
        type_.full_name()
    }
}

/// Appends ` (name = value)` to a description, or extends the parenthesis
/// the description ends with. Nothing is appended for no value or an empty
/// one, and a long value is cut.
pub(crate) fn append_optional_value(builder: &mut String, name: &str, value: Option<&str>) {
    let Some(value) = value.filter(|value| !value.is_empty()) else { return };

    append_name(builder, name);
    append_text(builder, value);
    builder.push(')');
}

/// [`append_optional_value`] for an untyped value: an object of the object
/// model is described by its own description, anything else by its text.
pub(crate) fn append_optional_boxed_value(
    builder: &mut String,
    name: &str,
    value: Option<&BoxedValue>,
    include_content: bool,
) {
    let Some(value) = value else { return };

    if let Some(object) = ValueTypes::as_object(&**value) {
        append_name(builder, name);
        build_debug_display(&object, builder, include_content);
        builder.push(')');
        return;
    }

    // Only an empty string is left out: another value with an empty text
    // is appended, as in the reference.
    let text = ValueTypes::to_display_string(Some(value));
    if text.is_empty() && is_string(value) {
        return;
    }

    append_name(builder, name);
    append_text(builder, &text);
    builder.push(')');
}

fn is_string(value: &BoxedValue) -> bool {
    (**value).downcast_ref::<String>().is_some() || (**value).downcast_ref::<&'static str>().is_some()
}

fn append_name(builder: &mut String, name: &str) {
    if builder.ends_with(')') {
        builder.pop();
        builder.push_str(", ");
    } else {
        builder.push_str(" (");
    }

    builder.push_str(name);
    builder.push_str(" = ");
}

fn append_text(builder: &mut String, value: &str) {
    const MAX_VALUE_LENGTH: usize = 50;

    // The reference counts UTF-16 code units.
    let units: Vec<u16> = value.encode_utf16().collect();
    if units.len() > MAX_VALUE_LENGTH {
        builder.push_str(&String::from_utf16_lossy(&units[..MAX_VALUE_LENGTH - 1]));
        builder.push('…');
    } else {
        builder.push_str(value);
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream project has no tests of the descriptions.
    use super::*;
    use crate::testing::{TestServices, UnitTestApplication};
    use crate::Control;
    use std::rc::Rc;

    fn describe(object: &FerroObject, include_content: bool) -> String {
        let mut builder = String::new();
        build_debug_display(object, &mut builder, include_content);
        builder
    }

    fn text(value: &str) -> Option<BoxedValue> {
        Some(Rc::new(value.to_string()))
    }

    #[test]
    fn content_controls_and_text_blocks_describe_their_content() {
        let _app = UnitTestApplication::start(TestServices::styled_window());

        let control = ContentControl::new();
        control.set_content(text("first"));
        assert_eq!("ContentControl", describe(&control, false));
        assert_eq!("ContentControl (Content = first)", describe(&control, true));

        let text_block = TextBlock::new();
        assert_eq!("TextBlock", describe(&text_block, true));
        text_block.set_text(Some("hello"));
        assert_eq!("TextBlock", describe(&text_block, false));
        assert_eq!("TextBlock (Text = hello)", describe(&text_block, true));

        // An object as the content is described by its own description.
        control.set_content(Some(Control::boxed(text_block)));
        assert_eq!("ContentControl (Content = TextBlock (Text = hello))", describe(&control, true));

        let headered = HeaderedContentControl::new();
        headered.set_content(text("body"));
        headered.set_header(text("title"));
        assert_eq!("HeaderedContentControl", describe(&headered, false));
        assert_eq!("HeaderedContentControl (Content = body, Header = title)", describe(&headered, true));

        let presenter = ContentPresenter::new();
        presenter.set_content(text("shown"));
        assert_eq!("ContentPresenter", describe(&presenter, false));
        assert_eq!("ContentPresenter (Content = shown)", describe(&presenter, true));
    }
}
