//! The clipboard actions (`Clipboard/`, namespace `FerroUI.Xaml.Interactions.Core`).

mod clear_clipboard_action;
mod get_clipboard_data_action;
mod get_clipboard_formats_action;
mod get_clipboard_text_action;
mod set_clipboard_data_object_action;
mod set_clipboard_text_action;

pub use clear_clipboard_action::ClearClipboardAction;
pub use get_clipboard_data_action::GetClipboardDataAction;
pub use get_clipboard_formats_action::GetClipboardFormatsAction;
pub use get_clipboard_text_action::GetClipboardTextAction;
pub use set_clipboard_data_object_action::SetClipboardDataObjectAction;
pub use set_clipboard_text_action::SetClipboardTextAction;

use crate::ClipboardRef;
use ferroui_base::input::platform::IClipboard;
use ferroui_base::{BoxedValue, Ref, Visual};
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use std::rc::Rc;

/// The sender of an action as a visual (`sender is Visual visual`).
pub(crate) fn visual_of(sender: Option<&BoxedValue>) -> Option<Ref<Visual>> {
    sender.and_then(object_of).and_then(|object| object.cast::<Visual>())
}

/// The clipboard an action uses: the one it is given, else the clipboard of the top level at the
/// root of the logical tree of `visual`
/// (`Clipboard ?? (visual.GetSelfAndLogicalAncestors().LastOrDefault() as TopLevel)?.Clipboard`).
pub(crate) fn resolve_clipboard(clipboard: Option<ClipboardRef>, visual: &Visual) -> Option<Rc<dyn IClipboard>> {
    if let Some(clipboard) = clipboard {
        return Some(clipboard.0);
    }

    visual
        .get_self_and_logical_ancestors()
        .last()
        .and_then(|root| root.cast::<TopLevel>())
        .and_then(|top_level| top_level.clipboard())
}
