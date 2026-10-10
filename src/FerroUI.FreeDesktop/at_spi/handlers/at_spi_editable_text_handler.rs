//! The port of `AtSpiEditableTextHandler.cs`: `org.a11y.atspi.EditableText`
//! over a value provider that can be written.
//!
//! Positions are counted in UTF-16 code units, as the reference counts
//! them (the indices of its strings).

use super::node_of;
use crate::at_spi::at_spi_constants::EDITABLE_TEXT_VERSION;
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::dbus::descriptions::EDITABLE_TEXT;
use crate::at_spi::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use ferroui_controls::automation::provider::IValueProvider;
use std::rc::{Rc, Weak};
use zbus::zvariant::Value;

pub(crate) struct AtSpiEditableTextHandler {
    node: Weak<AtSpiNode>,
}

/// `current.Insert(position, toInsert)` with the clamping of the handler.
pub(crate) fn insert_text(current: &str, position: i32, text: &str, length: i32) -> String {
    let mut units: Vec<u16> = current.encode_utf16().collect();
    let text: Vec<u16> = text.encode_utf16().collect();
    let position = usize::try_from(position).unwrap_or(0).min(units.len());
    let to_insert = match usize::try_from(length) {
        Ok(length) if length < text.len() => &text[..length],
        _ => &text[..],
    };
    units.splice(position..position, to_insert.iter().copied());
    String::from_utf16_lossy(&units)
}

/// `current.Remove(startPos, endPos - startPos)` with the clamping of
/// the handler; `None` for a range that is empty.
pub(crate) fn delete_text(current: &str, start_pos: i32, end_pos: i32) -> Option<String> {
    let mut units: Vec<u16> = current.encode_utf16().collect();
    let start_pos = usize::try_from(start_pos).unwrap_or(0).min(units.len());
    let end_pos = usize::try_from(end_pos).unwrap_or(0).min(units.len()).max(start_pos);
    if start_pos >= end_pos {
        return None;
    }

    units.drain(start_pos..end_pos);
    Some(String::from_utf16_lossy(&units))
}

impl AtSpiEditableTextHandler {
    pub(crate) fn new(node: Weak<AtSpiNode>) -> Self {
        Self { node }
    }

    /// The provider, when it can be written.
    fn provider(node: &AtSpiNode) -> Option<Rc<dyn IValueProvider>> {
        node.peer().get_provider::<dyn IValueProvider>().filter(|provider| !provider.is_read_only())
    }

    fn set(provider: &dyn IValueProvider, value: &str) -> Result<bool, DBusError> {
        provider.set_value(Some(value)).map_err(DBusError::failed)?;
        Ok(true)
    }

    fn set_text_contents(node: &AtSpiNode, new_contents: &str) -> Result<bool, DBusError> {
        let Some(provider) = Self::provider(node) else { return Ok(false) };
        Self::set(&*provider, new_contents)
    }

    fn insert_text(node: &AtSpiNode, position: i32, text: &str, length: i32) -> Result<bool, DBusError> {
        let Some(provider) = Self::provider(node) else { return Ok(false) };
        let current = provider.value().unwrap_or_default();
        Self::set(&*provider, &insert_text(&current, position, text, length))
    }

    fn delete_text(node: &AtSpiNode, start_pos: i32, end_pos: i32) -> Result<bool, DBusError> {
        let Some(provider) = Self::provider(node) else { return Ok(false) };
        let current = provider.value().unwrap_or_default();
        match delete_text(&current, start_pos, end_pos) {
            Some(new_value) => Self::set(&*provider, &new_value),
            None => Ok(false),
        }
    }
}

impl DBusInterface for AtSpiEditableTextHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &EDITABLE_TEXT
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let node = node_of(&self.node)?;
        match member {
            "SetTextContents" => reply((Self::set_text_contents(&node, &args::<String>(body)?)?,)),
            "InsertText" => {
                let (position, text, length) = args::<(i32, String, i32)>(body)?;
                reply((Self::insert_text(&node, position, &text, length)?,))
            }
            "CopyText" => {
                // Clipboard operations not supported via IValueProvider
                args::<(i32, i32)>(body)?;
                reply(())
            }
            "CutText" => {
                // Clipboard operations not supported via IValueProvider
                args::<(i32, i32)>(body)?;
                reply((false,))
            }
            "DeleteText" => {
                let (start_pos, end_pos) = args::<(i32, i32)>(body)?;
                reply((Self::delete_text(&node, start_pos, end_pos)?,))
            }
            "PasteText" => {
                // Clipboard operations not supported via IValueProvider
                args::<i32>(body)?;
                reply((false,))
            }
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        (name == "version").then(|| Value::from(EDITABLE_TEXT_VERSION))
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    #[test]
    fn text_is_inserted_at_a_clamped_position() {
        assert_eq!(insert_text("Hello", 5, " world", -1), "Hello world");
        assert_eq!(insert_text("Hello", 99, "!", 1), "Hello!");
        assert_eq!(insert_text("Hello", -4, ">", 5), ">Hello");
        // A length shorter than the text inserts its start.
        assert_eq!(insert_text("ab", 1, "XYZ", 2), "aXYb");
        assert_eq!(insert_text("ab", 1, "XYZ", 0), "ab");
        assert_eq!(insert_text("", 0, "x", -1), "x");
    }

    #[test]
    fn a_range_is_deleted() {
        assert_eq!(delete_text("Hello world", 5, 11), Some("Hello".to_string()));
        assert_eq!(delete_text("Hello", 3, 99), Some("Hel".to_string()));
        assert_eq!(delete_text("Hello", -2, 1), Some("ello".to_string()));
        assert_eq!(delete_text("Hello", 3, 3), None);
        assert_eq!(delete_text("Hello", 4, 2), None);
        assert_eq!(delete_text("", 0, 5), None);
    }
}
