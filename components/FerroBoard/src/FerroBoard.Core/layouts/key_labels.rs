// Derived from Layouts/KeyLabels.cs; see ../NOTICE.md.
use super::{ITextKeyData, KeyComputeContext, KeyDataError};
use crate::{
    editor::EnterAction,
    keys::*,
    settings::SpaceBarMode,
    text::{TextCulture, TextUtils, Utf16Text},
};
pub struct KeyLabels;
impl KeyLabels {
    pub fn get_display(
        data: &dyn ITextKeyData,
        ctx: &KeyComputeContext,
    ) -> Result<(Option<Utf16Text>, KeyIcon), KeyDataError> {
        let text = |s: &str| Ok((Some(s.into()), KeyIcon::None));
        let icon = |i| Ok((None, i));
        match data.data().code() {
            KeyCode::Space
                if !matches!(
                    ctx.mode(),
                    KeyboardMode::Characters | KeyboardMode::Symbols | KeyboardMode::Symbols2
                ) =>
            {
                return icon(KeyIcon::Space);
            }
            KeyCode::Space => {
                return match ctx.space_bar_mode() {
                    SpaceBarMode::CurrentLanguage => Ok((ctx.space_bar_label(), KeyIcon::None)),
                    SpaceBarMode::SpaceBarKey => icon(KeyIcon::Space),
                    _ => text(""),
                };
            }
            KeyCode::CjkSpace => return Ok((ctx.space_bar_label(), KeyIcon::None)),
            KeyCode::Enter => return icon(Self::get_enter_icon(ctx.enter_action())),
            KeyCode::Tab => return icon(KeyIcon::Tab),
            KeyCode::Shift => {
                return icon(match ctx.shift_state() {
                    ShiftState::CapsLock => KeyIcon::CapsLock,
                    ShiftState::ShiftedManual | ShiftState::ShiftedAutomatic => {
                        KeyIcon::ShiftActive
                    }
                    _ => KeyIcon::Shift,
                });
            }
            KeyCode::CapsLock => return icon(KeyIcon::CapsLock),
            KeyCode::Delete | KeyCode::DeleteWord => return icon(KeyIcon::Backspace),
            KeyCode::ForwardDelete | KeyCode::ForwardDeleteWord => {
                return icon(KeyIcon::DeleteForward);
            }
            KeyCode::ViewCharacters | KeyCode::ImeUiModeText => return text("ABC"),
            KeyCode::ViewSymbols => return text("?123"),
            KeyCode::ViewSymbols2 => return text("=\\<"),
            KeyCode::ViewNumeric | KeyCode::ViewNumericAdvanced | KeyCode::ViewPhone => {
                return text("123");
            }
            KeyCode::ViewPhone2 => return text("*#("),
            KeyCode::LanguageSwitch
            | KeyCode::ImeNextSubtype
            | KeyCode::ImePrevSubtype
            | KeyCode::ShowSubtypePicker
            | KeyCode::ImeSubtypePicker => return icon(KeyIcon::Language),
            KeyCode::ImeUiModeMedia => return icon(KeyIcon::Emoji),
            KeyCode::ImeUiModeClipboard => return icon(KeyIcon::Clipboard),
            KeyCode::ImeUiModeEditing => return icon(KeyIcon::TextEditing),
            KeyCode::Settings => return icon(KeyIcon::Settings),
            KeyCode::ImeHideUi => return icon(KeyIcon::HideKeyboard),
            KeyCode::VoiceInput => return icon(KeyIcon::Microphone),
            KeyCode::ArrowLeft => return icon(KeyIcon::ArrowLeft),
            KeyCode::ArrowRight => return icon(KeyIcon::ArrowRight),
            KeyCode::ArrowUp => return icon(KeyIcon::ArrowUp),
            KeyCode::ArrowDown => return icon(KeyIcon::ArrowDown),
            KeyCode::MoveStartOfLine => return icon(KeyIcon::Home),
            KeyCode::MoveStartOfPage => return icon(KeyIcon::DocumentStart),
            KeyCode::MoveEndOfLine => return icon(KeyIcon::End),
            KeyCode::MoveEndOfPage => return icon(KeyIcon::DocumentEnd),
            KeyCode::Undo => return icon(KeyIcon::Undo),
            KeyCode::Redo => return icon(KeyIcon::Redo),
            KeyCode::ClipboardCopy => return icon(KeyIcon::Copy),
            KeyCode::ClipboardCut => return icon(KeyIcon::Cut),
            KeyCode::ClipboardPaste => return icon(KeyIcon::Paste),
            KeyCode::ClipboardSelect => return icon(KeyIcon::Select),
            KeyCode::ClipboardSelectAll => return icon(KeyIcon::SelectAll),
            KeyCode::CompactLayoutToLeft => return icon(KeyIcon::OneHandedLeft),
            KeyCode::CompactLayoutToRight | KeyCode::ToggleCompactLayout => {
                return icon(KeyIcon::OneHandedRight);
            }
            KeyCode::SplitLayout | KeyCode::MergeLayout => return icon(KeyIcon::SplitKeyboard),
            KeyCode::ToggleIncognitoMode => return icon(KeyIcon::Incognito),
            KeyCode::ToggleAutocorrect => return icon(KeyIcon::Autocorrect),
            KeyCode::ToggleResizeMode => return icon(KeyIcon::ResizeHeight),
            KeyCode::ToggleFloatingWindow => return icon(KeyIcon::Floating),
            KeyCode::CharWidthSwitcher => {
                return text(if ctx.is_char_half_width() {
                    "全"
                } else {
                    "半"
                });
            }
            KeyCode::CharWidthFull => return text("全"),
            KeyCode::CharWidthHalf => return text("半"),
            KeyCode::KanaSwitcher => {
                return text(if ctx.is_kana_kata() {
                    "かな"
                } else {
                    "カナ"
                });
            }
            KeyCode::KanaHira => return text("かな"),
            KeyCode::KanaKata => return text("カナ"),
            KeyCode::KanaHalfKata => return text("ｶﾅ"),
            KeyCode::HalfSpace => return text("ZWNJ"),
            KeyCode::KanaSmall => return text("小"),
            _ => {}
        }
        if data.is_multi_text() {
            return Ok((
                if data
                    .data()
                    .label()
                    .as_ref()
                    .ok_or(KeyDataError::NullReference)?
                    .is_empty()
                {
                    data.as_string(true)?
                } else {
                    data.data().label().clone()
                },
                KeyIcon::None,
            ));
        }
        if KeyCode::is_character(data.data().code()) {
            if data.data().key_type() == KeyType::Numeric
                || data
                    .data()
                    .label()
                    .as_ref()
                    .ok_or(KeyDataError::NullReference)?
                    .is_empty()
                || TextUtils::get_code_points(
                    data.data()
                        .label()
                        .as_ref()
                        .ok_or(KeyDataError::NullReference)?,
                )
                .len()
                    == 1
            {
                return Ok((data.as_string(true)?, KeyIcon::None));
            }
            let mut units = data
                .data()
                .label()
                .as_ref()
                .ok_or(KeyDataError::NullReference)?
                .units()
                .to_vec();
            // Source capitalizes one UTF-16 char, not the first Unicode scalar.
            units[0] = TextCulture::invariant()
                .to_upper(&Utf16Text::from_units(vec![units[0]]))
                .units()[0];
            return Ok((Some(Utf16Text::from_units(units)), KeyIcon::None));
        }
        Ok((data.data().label().clone(), KeyIcon::None))
    }
    pub fn get_phone_sub_label(code: i32) -> Option<Utf16Text> {
        Some(
            match code {
                48 => "+",
                50 => "ABC",
                51 => "DEF",
                52 => "GHI",
                53 => "JKL",
                54 => "MNO",
                55 => "PQRS",
                56 => "TUV",
                57 => "WXYZ",
                _ => return None,
            }
            .into(),
        )
    }
    pub fn get_enter_icon(action: EnterAction) -> KeyIcon {
        match action {
            EnterAction::Go => KeyIcon::Go,
            EnterAction::Search => KeyIcon::Search,
            EnterAction::Send => KeyIcon::Send,
            EnterAction::Next => KeyIcon::Next,
            EnterAction::Previous => KeyIcon::Previous,
            EnterAction::Done => KeyIcon::Done,
            _ => KeyIcon::Enter,
        }
    }
}
