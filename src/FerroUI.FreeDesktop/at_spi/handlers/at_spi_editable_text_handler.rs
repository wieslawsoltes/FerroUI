//! The port of `AtSpiEditableTextHandler.cs`: `org.a11y.atspi.EditableText`
//! over a value provider that can be written.
//!
//! AT-SPI positions count Unicode scalars. InsertText's payload length is a
//! byte count in UTF-8.

use super::node_of;
use crate::at_spi::at_spi_constants::EDITABLE_TEXT_VERSION;
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::dbus::descriptions::EDITABLE_TEXT;
use crate::at_spi::dbus::interface::{
    args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription, MethodReply,
};
use ferroui_controls::automation::provider::{ITextProvider, IValueProvider};
use std::ops::Range;
use std::rc::{Rc, Weak};
use zbus::Variant;

pub(crate) struct AtSpiEditableTextHandler {
    node: Weak<AtSpiNode>,
}

fn byte_offset(text: &str, character: i32) -> usize {
    text.char_indices().nth(character.max(0) as usize).map_or(text.len(), |(byte, _)| byte)
}

fn insertion_prefix(text: &str, length: i32) -> &str {
    let mut length = usize::try_from(length).unwrap_or(text.len()).min(text.len());
    while !text.is_char_boundary(length) {
        length -= 1;
    }
    &text[..length]
}

fn insert_text(current: &str, position: i32, prefix: &str) -> String {
    let position = byte_offset(current, position);
    let mut result = current.to_owned();
    result.insert_str(position, prefix);
    result
}

fn delete_text(current: &str, start_pos: i32, end_pos: i32) -> Option<String> {
    let start = byte_offset(current, start_pos);
    let end = byte_offset(current, end_pos);
    if start >= end {
        return None;
    }
    let mut result = current.to_owned();
    result.replace_range(start..end, "");
    Some(result)
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

    fn set_text_contents_async(node: &AtSpiNode, new_contents: &str) -> Result<bool, DBusError> {
        let Some(provider) = Self::provider(node) else { return Ok(false) };
        Self::set(&*provider, new_contents)
    }

    fn version() -> u32 {
        EDITABLE_TEXT_VERSION
    }

    fn clipboard_call(&self, member: &str, body: &zbus::message::Body) -> Result<MethodReply, DBusError> {
        let node = node_of(&self.node)?;
        let copy = member == "CopyText";
        let paste = member == "PasteText";
        let (start, end) = if paste {
            let position = args::<i32>(body)?;
            (position, position)
        } else {
            args::<(i32, i32)>(body)?
        };
        let clipboard_reply = move |applied| {
            if copy { reply(()) } else { reply((applied,)) }
        };
        let Some(provider) = node.peer().get_provider::<dyn ITextProvider>() else {
            return Ok(clipboard_reply(false).into());
        };
        if start < 0 || end < start || (!paste && start == end) {
            return Ok(clipboard_reply(false).into());
        }
        let Some(start) = provider.utf16_offset_from_character(start as usize).map_err(DBusError::failed)? else {
            return Ok(clipboard_reply(false).into());
        };
        let operation = if paste {
            provider.paste_text_from_clipboard(start)
        } else {
            let Some(end) = provider.utf16_offset_from_character(end as usize).map_err(DBusError::failed)? else {
                return Ok(clipboard_reply(false).into());
            };
            provider.copy_text_to_clipboard(start..end, !copy)
        };
        Ok(MethodReply::Deferred(Box::pin(async move {
            clipboard_reply(operation.await.map_err(DBusError::failed)?)
        })))
    }

    fn replace_range(node: &AtSpiNode, positions: Range<i32>, text: &str) -> Result<Option<bool>, DBusError> {
        let Some(provider) = node.peer().get_provider::<dyn ITextProvider>() else { return Ok(None) };
        if provider.utf16_offset_from_character(0).map_err(DBusError::failed)?.is_none() {
            return Ok(None);
        }
        let offset = |position: i32| -> Result<Option<usize>, DBusError> {
            let native = provider.utf16_offset_from_character(position.max(0) as usize).map_err(DBusError::failed)?;
            if native.is_some() {
                return Ok(native);
            }
            let document = provider.document_range().map_err(DBusError::failed)?;
            Ok(document.text_offsets().map_err(DBusError::failed)?.map(|offsets| offsets.end))
        };
        let (Some(start), Some(end)) = (offset(positions.start)?, offset(positions.end)?) else {
            return Ok(None);
        };
        if start > end || (start == end && text.is_empty()) {
            return Ok(Some(false));
        }
        Ok(provider.replace_text(start..end, text).map_err(DBusError::failed)?.then_some(true))
    }

    fn insert_text_async(node: &AtSpiNode, position: i32, text: &str, length: i32) -> Result<bool, DBusError> {
        let Some(provider) = Self::provider(node) else { return Ok(false) };
        let prefix = insertion_prefix(text, length);
        if prefix.is_empty() {
            return Ok(true);
        }
        if let Some(changed) = Self::replace_range(node, position..position, prefix)? {
            return Ok(changed);
        }
        let current = provider.value().unwrap_or_default();
        Self::set(&*provider, &insert_text(&current, position, prefix))
    }

    fn delete_text_async(node: &AtSpiNode, start_pos: i32, end_pos: i32) -> Result<bool, DBusError> {
        let Some(provider) = Self::provider(node) else { return Ok(false) };
        if let Some(changed) = Self::replace_range(node, start_pos..end_pos, "")? {
            return Ok(changed);
        }
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

    fn dispatch(&self, member: &str, body: &zbus::message::Body) -> MethodReply {
        match member {
            "CopyText" | "CutText" | "PasteText" => {
                self.clipboard_call(member, body).unwrap_or_else(|error| MethodReply::Ready(Err(error)))
            }
            _ => self.call(member, body).into(),
        }
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let node = node_of(&self.node)?;
        match member {
            "SetTextContents" => reply((Self::set_text_contents_async(&node, &args::<String>(body)?)?,)),
            "InsertText" => {
                let (position, text, length) = args::<(i32, String, i32)>(body)?;
                reply((Self::insert_text_async(&node, position, &text, length)?,))
            }
            "DeleteText" => {
                let (start_pos, end_pos) = args::<(i32, i32)>(body)?;
                reply((Self::delete_text_async(&node, start_pos, end_pos)?,))
            }
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Variant<'static>> {
        (name == "version").then(|| Variant::from(Self::version()))
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    #[test]
    fn text_is_inserted_at_a_clamped_position() {
        let insert = |current, position, text, length| insert_text(current, position, insertion_prefix(text, length));
        assert_eq!(insert("Hello", 5, " world", -1), "Hello world");
        assert_eq!(insert("Hello", 99, "!", 1), "Hello!");
        assert_eq!(insert("Hello", -4, ">", 5), ">Hello");
        // A length shorter than the text inserts its start.
        assert_eq!(insert("ab", 1, "XYZ", 2), "aXYb");
        assert_eq!(insert("ab", 1, "XYZ", 0), "ab");
        assert_eq!(insert("", 0, "x", -1), "x");
        assert_eq!(insert("a😀b", 2, "é😀Z", 6), "a😀é😀b");
        assert_eq!(insert("a😀b", 2, "é😀Z", 5), "a😀éb");
    }

    #[test]
    fn a_range_is_deleted() {
        assert_eq!(delete_text("Hello world", 5, 11), Some("Hello".to_string()));
        assert_eq!(delete_text("Hello", 3, 99), Some("Hel".to_string()));
        assert_eq!(delete_text("Hello", -2, 1), Some("ello".to_string()));
        assert_eq!(delete_text("Hello", 3, 3), None);
        assert_eq!(delete_text("Hello", 4, 2), None);
        assert_eq!(delete_text("", 0, 5), None);
        assert_eq!(delete_text("a😀éz", 1, 3).as_deref(), Some("az"));
    }
}
