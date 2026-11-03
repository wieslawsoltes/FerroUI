// Derived from Editor/EditorController.cs; see ../NOTICE.md.
use super::{CapitalizationMode, EditorClock, ITextInputTarget, TargetReplaceError};
use crate::{
    layouts::PunctuationRule,
    text::{TextUtils, Utf16Text},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, SystemTime},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorError {
    Target(TargetReplaceError),
    NullReference,
    SubstringOutOfRange,
    InvalidClampRange,
    AbsoluteValueOverflow,
}
impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::Target(e) = self {
            return std::fmt::Display::fmt(e, f);
        }
        f.write_str(match self {
            Self::Target(_) => unreachable!("target case returned above"),
            Self::NullReference => "required editor data is missing",
            Self::SubstringOutOfRange => "SubstringOutOfRange",
            Self::InvalidClampRange => "InvalidClampRange",
            Self::AbsoluteValueOverflow => "AbsoluteValueOverflow",
        })
    }
}
impl std::error::Error for EditorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Target(error) = self {
            Some(error)
        } else {
            None
        }
    }
}
impl From<TargetReplaceError> for EditorError {
    fn from(e: TargetReplaceError) -> Self {
        Self::Target(e)
    }
}
pub type EditorResult<T> = Result<T, EditorError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditKind {
    None,
    Typing,
    Separator,
    Delete,
    Other,
}
#[derive(Clone)]
struct Snapshot {
    text: Utf16Text,
    selection_start: i32,
    selection_length: i32,
}
pub struct EditorController {
    target: Rc<dyn ITextInputTarget>,
    undo: RefCell<Vec<Snapshot>>,
    redo: RefCell<Vec<Snapshot>>,
    last_edit_time: Cell<Option<SystemTime>>,
    last_edit_kind: Cell<EditKind>,
    preferred_column: Cell<Option<i32>>,
    max_undo_steps: Cell<i32>,
    undo_group_timeout: Cell<Duration>,
    clock: RefCell<Option<EditorClock>>,
    selection_anchor: Cell<Option<i32>>,
}
fn substring(text: &Utf16Text, start: i32, length: i32) -> EditorResult<Utf16Text> {
    if start < 0
        || length < 0
        || start as usize > text.len()
        || length as usize > text.len() - start as usize
    {
        return Err(EditorError::SubstringOutOfRange);
    }
    if start == 0 && length as usize == text.len() {
        return Ok(text.clone());
    }
    Ok(Utf16Text::from_units(
        text.units()[start as usize..start as usize + length as usize].to_vec(),
    ))
}
impl PartialEq for EditorController {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for EditorController {}
impl EditorController {
    pub fn new(target: Rc<dyn ITextInputTarget>) -> Self {
        Self {
            target,
            undo: RefCell::new(Vec::new()),
            redo: RefCell::new(Vec::new()),
            last_edit_time: Cell::new(None),
            last_edit_kind: Cell::new(EditKind::None),
            preferred_column: Cell::new(None),
            max_undo_steps: Cell::new(100),
            undo_group_timeout: Cell::new(Duration::from_millis(1500)),
            clock: RefCell::new(Some(crate::editor::default_clock())),
            selection_anchor: Cell::new(None),
        }
    }
    pub fn target(&self) -> Rc<dyn ITextInputTarget> {
        self.target.clone()
    }
    pub fn max_undo_steps(&self) -> i32 {
        self.max_undo_steps.get()
    }
    pub fn set_max_undo_steps(&self, value: i32) {
        self.max_undo_steps.set(value);
    }
    pub fn undo_group_timeout(&self) -> Duration {
        self.undo_group_timeout.get()
    }
    pub fn set_undo_group_timeout(&self, value: Duration) {
        self.undo_group_timeout.set(value);
    }
    pub fn clock(&self) -> Option<EditorClock> {
        self.clock.borrow().clone()
    }
    pub fn set_clock(&self, value: EditorClock) {
        self.set_clock_nullable(Some(value));
    }
    pub fn set_clock_nullable(&self, value: Option<EditorClock>) {
        let previous = self.clock.replace(value);
        drop(previous);
    }
    pub fn selection_anchor(&self) -> Option<i32> {
        self.selection_anchor.get()
    }
    pub fn is_selecting(&self) -> bool {
        self.selection_anchor.get().is_some()
    }
    pub fn text(&self) -> Utf16Text {
        self.target.text()
    }
    pub fn selection_start(&self) -> i32 {
        let start = self.target.selection_start();
        start.clamp(0, self.text().len() as i32)
    }
    pub fn selection_length(&self) -> EditorResult<i32> {
        let length = self.target.selection_length();
        let text_length = self.text().len() as i32;
        let max = text_length.wrapping_sub(self.selection_start());
        if max < 0 {
            return Err(EditorError::InvalidClampRange);
        }
        Ok(length.clamp(0, max))
    }
    pub fn selection_end(&self) -> EditorResult<i32> {
        let start = self.selection_start();
        Ok(start.wrapping_add(self.selection_length()?))
    }
    pub fn has_selection(&self) -> EditorResult<bool> {
        Ok(self.selection_length()? > 0)
    }
    pub fn selected_text(&self) -> EditorResult<Utf16Text> {
        let text = self.text();
        let start = self.selection_start();
        substring(&text, start, self.selection_length()?)
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.borrow().is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.borrow().is_empty()
    }
    pub fn get_text_before_cursor(&self, max_length: i32) -> EditorResult<Utf16Text> {
        let start = self.selection_start();
        let from = start.wrapping_sub(max_length).max(0);
        substring(&self.text(), from, start.wrapping_sub(from))
    }
    pub fn get_text_after_cursor(&self, max_length: i32) -> EditorResult<Utf16Text> {
        let end = self.selection_end()?;
        let length = max_length.min((self.text().len() as i32).wrapping_sub(end));
        substring(&self.text(), end, length)
    }
    pub fn current_word(&self) -> EditorResult<Utf16Text> {
        if self.has_selection()? {
            Ok(Utf16Text::default())
        } else {
            Ok(TextUtils::get_current_word(
                &self.get_text_before_cursor(64)?,
            ))
        }
    }
    pub fn is_composing_word(&self) -> EditorResult<bool> {
        Ok(!self.current_word()?.is_empty()
            && !self
                .get_text_after_cursor(1)?
                .units()
                .first()
                .is_some_and(|c| TextUtils::is_word_core_char(*c)))
    }
    pub fn commit_text(&self, text: &Utf16Text) -> EditorResult<()> {
        self.commit_text_nullable(Some(text))
    }
    pub fn commit_text_nullable(&self, text: Option<&Utf16Text>) -> EditorResult<()> {
        let text = text.ok_or(EditorError::NullReference)?;
        if text.is_empty() && !self.has_selection()? {
            return Ok(());
        }
        self.record_undo(
            if text
                .units()
                .last()
                .is_some_and(|c| TextUtils::is_white_space(*c))
            {
                EditKind::Separator
            } else {
                EditKind::Typing
            },
        )?;
        let start = self.selection_start();
        let length = self.selection_length()?;
        self.target.replace(start, length, Some(text))?;
        self.end_selection_mode();
        self.preferred_column.set(None);
        Ok(())
    }
    pub fn replace_before_cursor(&self, count: i32, text: &Utf16Text) -> EditorResult<()> {
        self.replace_before_cursor_nullable(count, Some(text))
    }
    pub fn replace_before_cursor_nullable(
        &self,
        count: i32,
        text: Option<&Utf16Text>,
    ) -> EditorResult<()> {
        let start = self.selection_start().wrapping_sub(count).max(0);
        self.record_undo(EditKind::Typing)?;
        let length = self
            .selection_start()
            .wrapping_sub(start)
            .wrapping_add(self.selection_length()?);
        self.target.replace(start, length, text)?;
        self.preferred_column.set(None);
        Ok(())
    }
    pub fn replace_current_word(&self, replacement: &Utf16Text) -> EditorResult<()> {
        self.replace_current_word_nullable(Some(replacement))
    }
    pub fn replace_current_word_nullable(
        &self,
        replacement: Option<&Utf16Text>,
    ) -> EditorResult<()> {
        let word = self.current_word()?;
        self.record_undo(EditKind::Other)?;
        self.target.replace(
            self.selection_start().wrapping_sub(word.len() as i32),
            word.len() as i32,
            replacement,
        )?;
        self.preferred_column.set(None);
        Ok(())
    }
    pub fn delete_backward(&self) -> EditorResult<bool> {
        if self.has_selection()? {
            self.record_undo(EditKind::Delete)?;
            let start = self.selection_start();
            let length = self.selection_length()?;
            self.target
                .replace(start, length, Some(&Utf16Text::default()))?;
            self.end_selection_mode();
            return Ok(true);
        }
        let before = self.get_text_before_cursor(32)?;
        if before.is_empty() {
            return Ok(false);
        }
        let length = TextUtils::last_grapheme_length(&before);
        self.record_undo(EditKind::Delete)?;
        self.target.replace(
            self.selection_start().wrapping_sub(length),
            length,
            Some(&Utf16Text::default()),
        )?;
        self.preferred_column.set(None);
        Ok(true)
    }
    pub fn delete_word_backward(&self) -> EditorResult<bool> {
        if self.has_selection()? {
            return self.delete_backward();
        }
        let text = self.text();
        let start = TextUtils::find_previous_word_boundary(&text, self.selection_start());
        if start >= self.selection_start() {
            return Ok(false);
        }
        self.record_undo(EditKind::Other)?;
        self.target.replace(
            start,
            self.selection_start().wrapping_sub(start),
            Some(&Utf16Text::default()),
        )?;
        self.preferred_column.set(None);
        Ok(true)
    }
    pub fn delete_forward(&self) -> EditorResult<bool> {
        if self.has_selection()? {
            return self.delete_backward();
        }
        let after = self.get_text_after_cursor(32)?;
        if after.is_empty() {
            return Ok(false);
        }
        let length = TextUtils::first_grapheme_length(&after);
        self.record_undo(EditKind::Delete)?;
        self.target
            .replace(self.selection_start(), length, Some(&Utf16Text::default()))?;
        Ok(true)
    }
    pub fn delete_word_forward(&self) -> EditorResult<bool> {
        if self.has_selection()? {
            return self.delete_backward();
        }
        let text = self.text();
        let end = TextUtils::find_next_word_boundary(&text, self.selection_start());
        if end <= self.selection_start() {
            return Ok(false);
        }
        self.record_undo(EditKind::Other)?;
        self.target.replace(
            self.selection_start(),
            end.wrapping_sub(self.selection_start()),
            Some(&Utf16Text::default()),
        )?;
        Ok(true)
    }
    pub fn delete_characters_before(&self, count: i32) -> EditorResult<()> {
        let start = self.selection_start().wrapping_sub(count).max(0);
        if start == self.selection_start() {
            return Ok(());
        }
        self.record_undo(EditKind::Other)?;
        self.target.replace(
            start,
            self.selection_start().wrapping_sub(start),
            Some(&Utf16Text::default()),
        )?;
        Ok(())
    }
    pub fn move_horizontal(&self, delta: i32) -> EditorResult<()> {
        self.preferred_column.set(None);
        let text = self.text();
        let active = self.active_end()?;
        if !self.is_selecting() && self.has_selection()? {
            let pos = if delta < 0 {
                self.selection_start()
            } else {
                self.selection_end()?
            };
            self.set_caret(pos);
            return Ok(());
        }
        let mut target = active;
        let count = delta
            .checked_abs()
            .ok_or(EditorError::AbsoluteValueOverflow)?;
        for _ in 0..count {
            if delta < 0 {
                if target == 0 {
                    break;
                }
                let before = substring(&text, target.wrapping_sub(16).max(0), 16.min(target))?;
                target = target.wrapping_sub(TextUtils::last_grapheme_length(&before));
            } else {
                if target >= text.len() as i32 {
                    break;
                }
                let after = substring(
                    &text,
                    target,
                    16.min((text.len() as i32).wrapping_sub(target)),
                )?;
                target = target.wrapping_add(TextUtils::first_grapheme_length(&after));
            }
        }
        self.move_active_end_to(target);
        Ok(())
    }
    pub fn move_word(&self, direction: i32) -> EditorResult<()> {
        self.preferred_column.set(None);
        let text = self.text();
        let active = self.active_end()?;
        let pos = if direction < 0 {
            TextUtils::find_previous_word_boundary(&text, active)
        } else {
            TextUtils::find_next_word_boundary(&text, active)
        };
        self.move_active_end_to(pos);
        Ok(())
    }
    pub fn move_vertical(&self, direction: i32) -> EditorResult<()> {
        let text = self.text();
        let pos = self.active_end()?;
        let line_start = Self::line_start(&text, pos);
        let column = self
            .preferred_column
            .get()
            .unwrap_or_else(|| pos.wrapping_sub(line_start));
        let target = if direction < 0 {
            if line_start == 0 {
                0
            } else {
                let end = line_start - 1;
                let start = Self::line_start(&text, end);
                start.wrapping_add(column).min(end)
            }
        } else {
            let end = Self::line_end(&text, pos);
            if end >= text.len() as i32 {
                text.len() as i32
            } else {
                let start = end + 1;
                let end = Self::line_end(&text, start);
                start.wrapping_add(column).min(end)
            }
        };
        self.move_active_end_to(target);
        self.preferred_column.set(Some(column));
        Ok(())
    }
    pub fn move_line_start(&self) -> EditorResult<()> {
        self.preferred_column.set(None);
        let text = self.text();
        self.move_active_end_to(Self::line_start(&text, self.active_end()?));
        Ok(())
    }
    pub fn move_line_end(&self) -> EditorResult<()> {
        self.preferred_column.set(None);
        let text = self.text();
        self.move_active_end_to(Self::line_end(&text, self.active_end()?));
        Ok(())
    }
    pub fn move_document_start(&self) {
        self.preferred_column.set(None);
        self.move_active_end_to(0);
    }
    pub fn move_document_end(&self) {
        self.preferred_column.set(None);
        self.move_active_end_to(self.text().len() as i32);
    }
    pub fn set_caret(&self, position: i32) {
        self.target
            .select(position.clamp(0, self.text().len() as i32), 0);
    }
    pub fn select_all(&self) {
        self.selection_anchor.set(None);
        self.target.select(0, self.text().len() as i32);
    }
    pub fn toggle_selection_mode(&self) -> EditorResult<()> {
        if self.is_selecting() {
            self.end_selection_mode();
        } else {
            // Preserve the source's redundant conditional and getter side effects.
            let _has_selection = self.has_selection()?;
            self.selection_anchor.set(Some(self.selection_start()));
        }
        Ok(())
    }
    pub fn end_selection_mode(&self) {
        self.selection_anchor.set(None);
    }
    fn active_end(&self) -> EditorResult<i32> {
        if let Some(anchor) = self.selection_anchor.get()
            && self.has_selection()?
        {
            return if anchor == self.selection_start() {
                self.selection_end()
            } else {
                Ok(self.selection_start())
            };
        }
        Ok(self.selection_start())
    }
    fn move_active_end_to(&self, position: i32) {
        let position = position.clamp(0, self.text().len() as i32);
        if let Some(anchor) = self.selection_anchor.get() {
            let anchor = anchor.clamp(0, self.text().len() as i32);
            self.target
                .select(anchor.min(position), (position - anchor).abs());
        } else {
            self.target.select(position, 0);
        }
    }
    fn line_start(text: &Utf16Text, pos: i32) -> i32 {
        let pos = pos.clamp(0, text.len() as i32) as usize;
        text.units()[..pos]
            .iter()
            .rposition(|c| *c == 10)
            .map_or(0, |i| i as i32 + 1)
    }
    fn line_end(text: &Utf16Text, pos: i32) -> i32 {
        let pos = pos.clamp(0, text.len() as i32) as usize;
        text.units()[pos..]
            .iter()
            .position(|c| *c == 10)
            .map_or(text.len() as i32, |i| (pos + i) as i32)
    }
    pub fn break_undo_group(&self) {
        self.last_edit_kind.set(EditKind::None);
    }
    pub fn undo(&self) -> EditorResult<bool> {
        let Some(snapshot) = self.undo.borrow_mut().pop() else {
            return Ok(false);
        };
        let current = self.take_snapshot()?;
        self.redo.borrow_mut().push(current);
        self.restore(snapshot)?;
        self.last_edit_kind.set(EditKind::None);
        Ok(true)
    }
    pub fn redo(&self) -> EditorResult<bool> {
        let Some(snapshot) = self.redo.borrow_mut().pop() else {
            return Ok(false);
        };
        let current = self.take_snapshot()?;
        self.undo.borrow_mut().push(current);
        self.restore(snapshot)?;
        self.last_edit_kind.set(EditKind::None);
        Ok(true)
    }
    pub fn clear_history(&self) {
        self.undo.borrow_mut().clear();
        self.redo.borrow_mut().clear();
        self.last_edit_kind.set(EditKind::None);
    }
    fn record_undo(&self, kind: EditKind) -> EditorResult<()> {
        let now = self.clock().ok_or(EditorError::NullReference)?();
        let within_timeout = super::within_window(
            now,
            self.last_edit_time.get(),
            self.undo_group_timeout.get(),
        );
        let continues = within_timeout
            && (matches!(kind, EditKind::Typing | EditKind::Separator)
                && self.last_edit_kind.get() == EditKind::Typing
                || kind == EditKind::Delete && self.last_edit_kind.get() == EditKind::Delete);
        self.last_edit_time.set(Some(now));
        self.last_edit_kind.set(if kind == EditKind::Separator {
            EditKind::None
        } else {
            kind
        });
        if continues {
            return Ok(());
        }
        let snapshot = self.take_snapshot()?;
        self.undo.borrow_mut().push(snapshot);
        if self.undo.borrow().len() as i32 > self.max_undo_steps.get() {
            self.undo.borrow_mut().remove(0);
        }
        self.redo.borrow_mut().clear();
        Ok(())
    }
    fn take_snapshot(&self) -> EditorResult<Snapshot> {
        let text = self.text();
        let selection_start = self.selection_start();
        let selection_length = self.selection_length()?;
        Ok(Snapshot {
            text,
            selection_start,
            selection_length,
        })
    }
    fn restore(&self, snapshot: Snapshot) -> EditorResult<()> {
        self.target
            .replace(0, self.text().len() as i32, Some(&snapshot.text))?;
        self.target
            .select(snapshot.selection_start, snapshot.selection_length);
        Ok(())
    }
    pub fn should_auto_capitalize(
        &self,
        mode: CapitalizationMode,
        rule: &PunctuationRule,
    ) -> EditorResult<bool> {
        self.should_auto_capitalize_nullable(mode, Some(rule))
    }
    /// Runtime null bridge; short-circuit paths do not dereference the rule.
    pub fn should_auto_capitalize_nullable(
        &self,
        mode: CapitalizationMode,
        rule: Option<&PunctuationRule>,
    ) -> EditorResult<bool> {
        if mode == CapitalizationMode::None || self.has_selection()? {
            return Ok(false);
        }
        if mode == CapitalizationMode::Characters {
            return Ok(true);
        }
        let before = self.get_text_before_cursor(128)?;
        let Some(last) = before.units().last().copied() else {
            return Ok(true);
        };
        if mode == CapitalizationMode::Words {
            return Ok(TextUtils::is_white_space(last));
        }
        if last == 10 {
            return Ok(true);
        }
        if !TextUtils::is_white_space(last) {
            return Ok(false);
        }
        let end = before
            .units()
            .iter()
            .rposition(|c| !TextUtils::is_white_space(*c))
            .map_or(0, |i| i + 1);
        if end == 0 {
            return Ok(true);
        }
        let mut i = end - 1;
        while i > 0 && matches!(before.units()[i], 34 | 39 | 41 | 93 | 187 | 0x201d | 0x2019) {
            i -= 1;
        }
        Ok(rule
            .ok_or(EditorError::NullReference)?
            .symbols_terminating_sentence()
            .ok_or(EditorError::NullReference)?
            .units()
            .contains(&before.units()[i]))
    }
}
