// Generated from KeyboardStrings.cs; see NOTICE.md.
use ferroboard_core::text::Utf16Text;
use std::sync::{OnceLock, RwLock};
pub struct KeyboardStrings;
fn values() -> &'static RwLock<Vec<Option<Utf16Text>>> {
    static VALUES: OnceLock<RwLock<Vec<Option<Utf16Text>>>> = OnceLock::new();
    VALUES.get_or_init(|| {
        RwLock::new(vec![
            Some(Utf16Text::from("Shift")),
            Some(Utf16Text::from("Backspace")),
            Some(Utf16Text::from("Enter")),
            Some(Utf16Text::from("Space")),
            Some(Utf16Text::from("Switch language")),
            Some(Utf16Text::from("Emoji")),
            Some(Utf16Text::from("Search emoji")),
            Some(Utf16Text::from("Emoticons")),
            Some(Utf16Text::from("Recently used emojis appear here")),
            Some(Utf16Text::from("No emoji found")),
            Some(Utf16Text::from("Clipboard")),
            Some(Utf16Text::from("Disable clipboard history")),
            Some(Utf16Text::from("Enable clipboard history")),
            Some(Utf16Text::from("Clear clipboard history")),
            Some(Utf16Text::from(
                "Copied text appears here. Pin items to keep them.",
            )),
            Some(Utf16Text::from("Clipboard history is turned off.")),
            Some(Utf16Text::from("Pin")),
            Some(Utf16Text::from("Unpin")),
            Some(Utf16Text::from("Delete")),
            Some(Utf16Text::from("Remove")),
            Some(Utf16Text::from("Line start")),
            Some(Utf16Text::from("Up")),
            Some(Utf16Text::from("Line end")),
            Some(Utf16Text::from("Left")),
            Some(Utf16Text::from("Select")),
            Some(Utf16Text::from("Right")),
            Some(Utf16Text::from("Start of text")),
            Some(Utf16Text::from("Down")),
            Some(Utf16Text::from("End of text")),
            Some(Utf16Text::from("Select all")),
            Some(Utf16Text::from("Copy")),
            Some(Utf16Text::from("Cut")),
            Some(Utf16Text::from("Paste")),
            Some(Utf16Text::from("Undo")),
            Some(Utf16Text::from("Redo")),
            Some(Utf16Text::from("Exit one-handed mode")),
            Some(Utf16Text::from("Move keyboard left")),
            Some(Utf16Text::from("Move keyboard right")),
            Some(Utf16Text::from("Drag up or down to resize")),
            Some(Utf16Text::from("Reset")),
            Some(Utf16Text::from("Done")),
            Some(Utf16Text::from("Back")),
            Some(Utf16Text::from("Hide keyboard")),
            Some(Utf16Text::from("Quick actions")),
            Some(Utf16Text::from("Manage languages")),
            Some(Utf16Text::from("Recent")),
            Some(Utf16Text::from("Number pad")),
            Some(Utf16Text::from("Text editing")),
            Some(Utf16Text::from("One-handed mode")),
            Some(Utf16Text::from("Split keyboard")),
            Some(Utf16Text::from("Floating keyboard")),
            Some(Utf16Text::from("Resize keyboard")),
            Some(Utf16Text::from("Incognito")),
            Some(Utf16Text::from("Auto-correct")),
            Some(Utf16Text::from("Languages")),
            Some(Utf16Text::from("Settings")),
            Some(Utf16Text::from("ABC")),
            Some(Utf16Text::from("Paste {0}")),
        ])
    })
}
impl KeyboardStrings {
    pub fn shift() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[0]
            .clone()
    }
    pub fn set_shift(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[0] = value;
    }
    pub fn backspace() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[1]
            .clone()
    }
    pub fn set_backspace(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[1] = value;
    }
    pub fn enter() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[2]
            .clone()
    }
    pub fn set_enter(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[2] = value;
    }
    pub fn space() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[3]
            .clone()
    }
    pub fn set_space(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[3] = value;
    }
    pub fn switch_language() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[4]
            .clone()
    }
    pub fn set_switch_language(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[4] = value;
    }
    pub fn emoji() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[5]
            .clone()
    }
    pub fn set_emoji(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[5] = value;
    }
    pub fn search_emoji() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[6]
            .clone()
    }
    pub fn set_search_emoji(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[6] = value;
    }
    pub fn emoticons() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[7]
            .clone()
    }
    pub fn set_emoticons(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[7] = value;
    }
    pub fn recent_emoji_empty() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[8]
            .clone()
    }
    pub fn set_recent_emoji_empty(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[8] = value;
    }
    pub fn no_emoji_found() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[9]
            .clone()
    }
    pub fn set_no_emoji_found(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[9] = value;
    }
    pub fn clipboard() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[10]
            .clone()
    }
    pub fn set_clipboard(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[10] = value;
    }
    pub fn disable_clipboard_history() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[11]
            .clone()
    }
    pub fn set_disable_clipboard_history(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[11] = value;
    }
    pub fn enable_clipboard_history() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[12]
            .clone()
    }
    pub fn set_enable_clipboard_history(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[12] = value;
    }
    pub fn clear_clipboard_history() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[13]
            .clone()
    }
    pub fn set_clear_clipboard_history(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[13] = value;
    }
    pub fn clipboard_empty() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[14]
            .clone()
    }
    pub fn set_clipboard_empty(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[14] = value;
    }
    pub fn clipboard_disabled() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[15]
            .clone()
    }
    pub fn set_clipboard_disabled(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[15] = value;
    }
    pub fn pin() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[16]
            .clone()
    }
    pub fn set_pin(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[16] = value;
    }
    pub fn unpin() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[17]
            .clone()
    }
    pub fn set_unpin(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[17] = value;
    }
    pub fn delete() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[18]
            .clone()
    }
    pub fn set_delete(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[18] = value;
    }
    pub fn remove() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[19]
            .clone()
    }
    pub fn set_remove(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[19] = value;
    }
    pub fn line_start() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[20]
            .clone()
    }
    pub fn set_line_start(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[20] = value;
    }
    pub fn up() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[21]
            .clone()
    }
    pub fn set_up(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[21] = value;
    }
    pub fn line_end() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[22]
            .clone()
    }
    pub fn set_line_end(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[22] = value;
    }
    pub fn left() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[23]
            .clone()
    }
    pub fn set_left(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[23] = value;
    }
    pub fn select() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[24]
            .clone()
    }
    pub fn set_select(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[24] = value;
    }
    pub fn right() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[25]
            .clone()
    }
    pub fn set_right(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[25] = value;
    }
    pub fn start_of_text() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[26]
            .clone()
    }
    pub fn set_start_of_text(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[26] = value;
    }
    pub fn down() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[27]
            .clone()
    }
    pub fn set_down(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[27] = value;
    }
    pub fn end_of_text() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[28]
            .clone()
    }
    pub fn set_end_of_text(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[28] = value;
    }
    pub fn select_all() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[29]
            .clone()
    }
    pub fn set_select_all(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[29] = value;
    }
    pub fn copy() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[30]
            .clone()
    }
    pub fn set_copy(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[30] = value;
    }
    pub fn cut() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[31]
            .clone()
    }
    pub fn set_cut(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[31] = value;
    }
    pub fn paste() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[32]
            .clone()
    }
    pub fn set_paste(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[32] = value;
    }
    pub fn undo() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[33]
            .clone()
    }
    pub fn set_undo(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[33] = value;
    }
    pub fn redo() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[34]
            .clone()
    }
    pub fn set_redo(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[34] = value;
    }
    pub fn exit_one_handed() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[35]
            .clone()
    }
    pub fn set_exit_one_handed(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[35] = value;
    }
    pub fn move_keyboard_left() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[36]
            .clone()
    }
    pub fn set_move_keyboard_left(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[36] = value;
    }
    pub fn move_keyboard_right() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[37]
            .clone()
    }
    pub fn set_move_keyboard_right(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[37] = value;
    }
    pub fn resize_hint() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[38]
            .clone()
    }
    pub fn set_resize_hint(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[38] = value;
    }
    pub fn reset() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[39]
            .clone()
    }
    pub fn set_reset(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[39] = value;
    }
    pub fn done() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[40]
            .clone()
    }
    pub fn set_done(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[40] = value;
    }
    pub fn back() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[41]
            .clone()
    }
    pub fn set_back(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[41] = value;
    }
    pub fn hide_keyboard() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[42]
            .clone()
    }
    pub fn set_hide_keyboard(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[42] = value;
    }
    pub fn quick_actions() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[43]
            .clone()
    }
    pub fn set_quick_actions(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[43] = value;
    }
    pub fn manage_languages() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[44]
            .clone()
    }
    pub fn set_manage_languages(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[44] = value;
    }
    pub fn recent() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[45]
            .clone()
    }
    pub fn set_recent(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[45] = value;
    }
    pub fn number_pad() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[46]
            .clone()
    }
    pub fn set_number_pad(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[46] = value;
    }
    pub fn text_editing() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[47]
            .clone()
    }
    pub fn set_text_editing(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[47] = value;
    }
    pub fn one_handed_mode() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[48]
            .clone()
    }
    pub fn set_one_handed_mode(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[48] = value;
    }
    pub fn split_keyboard() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[49]
            .clone()
    }
    pub fn set_split_keyboard(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[49] = value;
    }
    pub fn floating_keyboard() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[50]
            .clone()
    }
    pub fn set_floating_keyboard(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[50] = value;
    }
    pub fn resize_keyboard() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[51]
            .clone()
    }
    pub fn set_resize_keyboard(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[51] = value;
    }
    pub fn incognito() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[52]
            .clone()
    }
    pub fn set_incognito(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[52] = value;
    }
    pub fn auto_correct() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[53]
            .clone()
    }
    pub fn set_auto_correct(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[53] = value;
    }
    pub fn languages() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[54]
            .clone()
    }
    pub fn set_languages(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[54] = value;
    }
    pub fn settings() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[55]
            .clone()
    }
    pub fn set_settings(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[55] = value;
    }
    pub fn abc() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[56]
            .clone()
    }
    pub fn set_abc(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[56] = value;
    }
    pub fn paste_format() -> Option<Utf16Text> {
        values()
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[57]
            .clone()
    }
    pub fn set_paste_format(value: Option<Utf16Text>) {
        values()
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[57] = value;
    }
    #[cfg(test)]
    pub(crate) fn source_value(name: &str) -> Option<Option<Utf16Text>> {
        match name {
            "Shift" => Some(Self::shift()),
            "Backspace" => Some(Self::backspace()),
            "Enter" => Some(Self::enter()),
            "Space" => Some(Self::space()),
            "SwitchLanguage" => Some(Self::switch_language()),
            "Emoji" => Some(Self::emoji()),
            "SearchEmoji" => Some(Self::search_emoji()),
            "Emoticons" => Some(Self::emoticons()),
            "RecentEmojiEmpty" => Some(Self::recent_emoji_empty()),
            "NoEmojiFound" => Some(Self::no_emoji_found()),
            "Clipboard" => Some(Self::clipboard()),
            "DisableClipboardHistory" => Some(Self::disable_clipboard_history()),
            "EnableClipboardHistory" => Some(Self::enable_clipboard_history()),
            "ClearClipboardHistory" => Some(Self::clear_clipboard_history()),
            "ClipboardEmpty" => Some(Self::clipboard_empty()),
            "ClipboardDisabled" => Some(Self::clipboard_disabled()),
            "Pin" => Some(Self::pin()),
            "Unpin" => Some(Self::unpin()),
            "Delete" => Some(Self::delete()),
            "Remove" => Some(Self::remove()),
            "LineStart" => Some(Self::line_start()),
            "Up" => Some(Self::up()),
            "LineEnd" => Some(Self::line_end()),
            "Left" => Some(Self::left()),
            "Select" => Some(Self::select()),
            "Right" => Some(Self::right()),
            "StartOfText" => Some(Self::start_of_text()),
            "Down" => Some(Self::down()),
            "EndOfText" => Some(Self::end_of_text()),
            "SelectAll" => Some(Self::select_all()),
            "Copy" => Some(Self::copy()),
            "Cut" => Some(Self::cut()),
            "Paste" => Some(Self::paste()),
            "Undo" => Some(Self::undo()),
            "Redo" => Some(Self::redo()),
            "ExitOneHanded" => Some(Self::exit_one_handed()),
            "MoveKeyboardLeft" => Some(Self::move_keyboard_left()),
            "MoveKeyboardRight" => Some(Self::move_keyboard_right()),
            "ResizeHint" => Some(Self::resize_hint()),
            "Reset" => Some(Self::reset()),
            "Done" => Some(Self::done()),
            "Back" => Some(Self::back()),
            "HideKeyboard" => Some(Self::hide_keyboard()),
            "QuickActions" => Some(Self::quick_actions()),
            "ManageLanguages" => Some(Self::manage_languages()),
            "Recent" => Some(Self::recent()),
            "NumberPad" => Some(Self::number_pad()),
            "TextEditing" => Some(Self::text_editing()),
            "OneHandedMode" => Some(Self::one_handed_mode()),
            "SplitKeyboard" => Some(Self::split_keyboard()),
            "FloatingKeyboard" => Some(Self::floating_keyboard()),
            "ResizeKeyboard" => Some(Self::resize_keyboard()),
            "Incognito" => Some(Self::incognito()),
            "AutoCorrect" => Some(Self::auto_correct()),
            "Languages" => Some(Self::languages()),
            "Settings" => Some(Self::settings()),
            "Abc" => Some(Self::abc()),
            "PasteFormat" => Some(Self::paste_format()),
            _ => None,
        }
    }
}
