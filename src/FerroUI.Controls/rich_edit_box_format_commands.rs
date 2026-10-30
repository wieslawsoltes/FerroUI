//! Formatting commands used by the rich editor's themed command flyout.
//!
//! Source: microsoft-ui-xaml@fe244ca6ac4d482712b234a9de3cf771cc6481de,
//! controls/dev/CommandBarFlyout/TextCommandBarFlyout.cpp, ExecuteBoldCommand,
//! ExecuteItalicCommand and ExecuteUnderlineCommand. The native commands query
//! the selection and turn a uniformly enabled attribute off; a mixed attribute
//! becomes enabled. Queries therefore inspect each selected text run, independently
//! for each attribute, rather than using the editor's insertion-format getter.

use super::*;
use crate::documents::{Inline, InlineCollection, Paragraph, Span};
use std::ops::Range;

#[cfg(test)]
#[path = "rich_edit_format_command_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FormattingState {
    bold: Option<bool>,
    italic: Option<bool>,
    underline: Option<bool>,
}

impl Default for FormattingState {
    fn default() -> Self {
        Self {
            bold: Some(false),
            italic: Some(false),
            underline: Some(false),
        }
    }
}

struct FormattingRun {
    range: Range<usize>,
    bold: bool,
    italic: bool,
    decorations: Option<TextDecorationCollection>,
}

impl FormattingRun {
    fn underlined(&self) -> bool {
        self.decorations.as_ref().is_some_and(|items| {
            items
                .list()
                .snapshot()
                .iter()
                .any(|item| item.location() == TextDecorationLocation::Underline)
        })
    }

    fn state(&self) -> FormattingState {
        FormattingState {
            bold: Some(self.bold),
            italic: Some(self.italic),
            underline: Some(self.underlined()),
        }
    }

    fn with_underline(&self, enabled: bool) -> CharacterFormat {
        let mut decorations = self.decorations.as_ref().map_or_else(Vec::new, |items| {
            items
                .list()
                .snapshot()
                .iter()
                .filter(|item| item.location() != TextDecorationLocation::Underline)
                .cloned()
                .collect()
        });
        if enabled {
            decorations.push(TextDecorations::underline().get(0));
        }
        CharacterFormat {
            text_decorations: Some(
                (!decorations.is_empty())
                    .then(|| TextDecorationCollection::from_items(decorations)),
            ),
            ..CharacterFormat::default()
        }
    }
}

fn merge_state(state: &mut Option<FormattingState>, next: FormattingState) {
    if let Some(current) = state {
        if current.bold != next.bold {
            current.bold = None;
        }
        if current.italic != next.italic {
            current.italic = None;
        }
        if current.underline != next.underline {
            current.underline = None;
        }
    } else {
        *state = Some(next);
    }
}

impl RichEditBox {
    /// Whether the UI should enable formatting commands. Programmatic formatting
    /// retains the normal editor error contract; disabled controls are not editable
    /// through their command flyout.
    pub fn can_format(&self) -> bool {
        !self.is_read_only() && self.is_effectively_enabled()
    }

    /// `None` denotes mixed bold formatting across the selection.
    pub fn bold_state(&self) -> Option<bool> {
        self.selection_formatting_state().bold
    }
    /// `None` denotes mixed italic formatting across the selection.
    pub fn italic_state(&self) -> Option<bool> {
        self.selection_formatting_state().italic
    }
    /// `None` denotes mixed underline formatting across the selection.
    pub fn underline_state(&self) -> Option<bool> {
        self.selection_formatting_state().underline
    }
    /// Checked only when every selected text run is bold.
    pub fn is_bold(&self) -> bool {
        self.bold_state() == Some(true)
    }
    /// Checked only when every selected text run is italic.
    pub fn is_italic(&self) -> bool {
        self.italic_state() == Some(true)
    }
    /// Checked only when every selected text run is underlined.
    pub fn is_underlined(&self) -> bool {
        self.underline_state() == Some(true)
    }

    fn formatting_runs(&self) -> Vec<FormattingRun> {
        let range = self.selection().range();
        if range.is_empty() {
            let current = self.editor().current_character_format().unwrap_or_default();
            return vec![FormattingRun {
                range,
                bold: current.font_weight == Some(FontWeight::Bold),
                italic: current.font_style == Some(FontStyle::Italic),
                decorations: current.text_decorations.flatten(),
            }];
        }
        fn visit(
            collection: &InlineCollection,
            offset: &mut usize,
            selection: &Range<usize>,
            runs: &mut Vec<FormattingRun>,
        ) {
            for inline in collection.snapshot().iter() {
                if *offset >= selection.end {
                    break;
                }
                if let Some(span) = inline.cast::<Span>() {
                    visit(&span.inlines(), offset, selection, runs);
                    continue;
                }
                // Inline's virtual text projection also supports custom inlines;
                // strings are limited to individual runs, never the entire story.
                let mut text = String::new();
                inline.append_text(&mut text);
                let end = *offset + text.encode_utf16().count();
                if *offset < selection.end && end > selection.start {
                    runs.push(FormattingRun {
                        range: (*offset).max(selection.start)..end.min(selection.end),
                        bold: inline.font_weight() == FontWeight::Bold,
                        italic: inline.font_style() == FontStyle::Italic,
                        decorations: inline.text_decorations(),
                    });
                }
                *offset = end;
            }
        }
        let mut runs = Vec::new();
        let mut offset = 0;
        for block in self.blocks().snapshot().iter() {
            if offset >= range.end {
                break;
            }
            let start = offset;
            if let Some(paragraph) = block.cast::<Paragraph>() {
                visit(&paragraph.inlines(), &mut offset, &range, &mut runs);
            } else {
                offset += block.text_length();
            }
            // Empty paragraphs own their character defaults; document formatting
            // applies the selected paragraph mark to those defaults.
            if offset == start && range.start <= offset && range.end > offset {
                runs.push(FormattingRun {
                    range: offset..offset + 1,
                    bold: block.font_weight() == FontWeight::Bold,
                    italic: block.font_style() == FontStyle::Italic,
                    decorations: block.get_value(Inline::text_decorations_property()),
                });
            }
            offset += 1;
        }
        runs
    }

    fn selection_formatting_state(&self) -> FormattingState {
        let mut state = None;
        for run in self.formatting_runs() {
            merge_state(&mut state, run.state());
        }
        state.unwrap_or_default()
    }

    fn prepare_format_command(&self) -> Result<Rc<RichTextEditor>, EditError> {
        let editor = self.editor();
        if editor.is_read_only() {
            return Err(EditError::ReadOnly);
        }
        if editor.is_in_operation() {
            return Err(EditError::ReentrantEdit);
        }
        // Formatting commits no preedit text. As with other editor operations,
        // cancel the preview before inspecting the accepted selection/runs.
        self.cancel_composition()?;
        let editor = self.editor();
        self.synchronize_editor_selection(&editor)?;
        Ok(editor)
    }

    /// Toggle bold for the entire selection; a mixed selection becomes bold.
    pub fn toggle_bold(&self) -> Result<(), EditError> {
        let editor = self.prepare_format_command()?;
        editor.apply_format(&CharacterFormat {
            font_weight: Some(if self.is_bold() {
                FontWeight::Normal
            } else {
                FontWeight::Bold
            }),
            ..CharacterFormat::default()
        })
    }

    /// Toggle italic for the entire selection; a mixed selection becomes italic.
    pub fn toggle_italic(&self) -> Result<(), EditError> {
        let editor = self.prepare_format_command()?;
        editor.apply_format(&CharacterFormat {
            font_style: Some(if self.is_italic() {
                FontStyle::Normal
            } else {
                FontStyle::Italic
            }),
            ..CharacterFormat::default()
        })
    }

    /// Toggle underline in one undo transaction. Existing strike-through,
    /// overline and other decoration data remain specific to each selected run.
    pub fn toggle_underline(&self) -> Result<(), EditError> {
        let editor = self.prepare_format_command()?;
        let runs = self.formatting_runs();
        let enabled = !runs.iter().all(FormattingRun::underlined);
        let selection = editor.selection();
        if selection.is_empty() {
            if let Some(run) = runs.first() {
                editor.apply_format(&run.with_underline(enabled))?;
            }
            return Ok(());
        }
        editor.edit_group(|editor| {
            for run in &runs {
                if run.underlined() == enabled {
                    continue;
                }
                editor.set_selection(EditorSelection::new(run.range.start, run.range.end))?;
                editor.apply_format(&run.with_underline(enabled))?;
            }
            editor.set_selection(selection)
        })
    }

    pub(super) fn update_format_command_states(&self) {
        self.set_and_raise_cell(
            Self::can_format_property(),
            &self.can_format,
            self.can_format(),
        );
        let next = self.selection_formatting_state();
        let previous = self.formatting_state.replace(next);
        macro_rules! notify {
            ($property:ident, $old:expr, $new:expr) => {
                if self.formatting_state.get() != next {
                    return;
                }
                if ($old) != ($new) {
                    self.raise_direct_property_changed(Self::$property(), &$old, &$new);
                }
            };
        }
        notify!(bold_state_property, previous.bold, next.bold);
        notify!(italic_state_property, previous.italic, next.italic);
        notify!(underline_state_property, previous.underline, next.underline);
        notify!(
            is_bold_property,
            previous.bold == Some(true),
            next.bold == Some(true)
        );
        notify!(
            is_italic_property,
            previous.italic == Some(true),
            next.italic == Some(true)
        );
        notify!(
            is_underlined_property,
            previous.underline == Some(true),
            next.underline == Some(true)
        );
    }
}
