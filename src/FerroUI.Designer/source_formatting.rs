use super::{DesignerWorkspace, byte_from_utf16};
use ferroui_controls::{CheckBox, ComboBox, TextBox};
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_language::{AttributeLayout, FormattingOptions, LineIndex, SelfClosingSpace};
use std::rc::Rc;

#[cfg(test)]
#[path = "source_formatting_tests.rs"]
mod tests;

impl DesignerWorkspace {
    pub(crate) fn source_formatting_options(&self) -> Result<FormattingOptions, String> {
        let selected = |name| self.root.get_control::<ComboBox>(name).selected_index();
        let checked = |name| self.root.get_control::<CheckBox>(name).is_checked() == Some(true);
        let number = |name, hint| {
            self.root
                .get_control::<TextBox>(name)
                .text()
                .unwrap_or_default()
                .trim()
                .parse::<u32>()
                .map_err(|_| hint)
        };
        let attribute_layout = match selected("SourceFormatAttributes") {
            1 => AttributeLayout::Inline,
            2 => AttributeLayout::OnePerLine,
            3 => AttributeLayout::Wrap,
            _ => AttributeLayout::Preserve,
        };
        let options = FormattingOptions {
            attribute_layout,
            wrap_column: if attribute_layout == AttributeLayout::Wrap {
                number(
                    "SourceFormatWrapColumn",
                    "Enter a wrap column from 20 through 240",
                )?
            } else {
                FormattingOptions::default().wrap_column
            },
            tab_size: number("SourceFormatTabSize", "Enter a tab size from 1 through 16")?,
            insert_spaces: selected("SourceFormatIndent") == 0,
            closing_bracket_on_new_line: checked("SourceFormatBracketLine"),
            self_closing_space: match selected("SourceFormatClosingSpace") {
                1 => SelfClosingSpace::Space,
                2 => SelfClosingSpace::None,
                _ => SelfClosingSpace::Preserve,
            },
            preserve_blank_lines: checked("SourceFormatBlankLines"),
            trim_trailing_whitespace: checked("SourceFormatTrailingWhitespace"),
            insert_final_newline: checked("SourceFormatFinalNewline"),
            trim_final_newlines: checked("SourceFormatFinalBlankLines"),
        };
        options.validate().map_err(|error| error.to_string())?;
        Ok(options)
    }

    pub(super) fn format_source(
        self: &Rc<Self>,
        uri: &str,
        source: &str,
        revision: u64,
    ) -> Result<(), String> {
        let options = self.source_formatting_options()?;
        let first = byte_from_utf16(source, self.source.selection_start());
        let last = byte_from_utf16(source, self.source.selection_end());
        let selection = SourceSpan::new(first.min(last), first.max(last));
        let index = LineIndex::new(source);
        let edits = {
            let tools = self.source_tools.borrow();
            if selection.is_empty() {
                tools.service.format(uri, options)
            } else {
                tools.service.format_range(
                    uri,
                    index.range(selection).map_err(|error| error.to_string())?,
                    options,
                )
            }
        }
        .map_err(|error| error.to_string())?;
        let patches = edits
            .into_iter()
            .map(|edit| {
                Ok(TextPatch {
                    span: index.span(edit.range).map_err(|error| error.to_string())?,
                    replacement: edit.new_text,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if patches.is_empty() {
            return Ok(());
        }
        let retained = SourceSpan::new(
            after_format(selection.start, &patches),
            after_format(selection.end, &patches),
        );
        {
            let mut session = self.session.borrow_mut();
            if session.document().revision() != revision {
                return Err("Use the current XAML buffer before formatting".into());
            }
            session
                .commit_xaml_patches("Format XAML".into(), patches)
                .map_err(|error| error.to_string())?;
            session.changed(true);
        }
        self.pending_source.set(false);
        self.refresh(true);
        self.select_source_span(retained);
        Ok(())
    }
}

fn after_format(offset: usize, patches: &[TextPatch]) -> usize {
    let mut shift = 0isize;
    for patch in patches
        .iter()
        .take_while(|patch| patch.span.start <= offset)
    {
        if offset < patch.span.end {
            return patch.span.start.saturating_add_signed(shift)
                + (offset - patch.span.start).min(patch.replacement.len());
        }
        shift += patch.replacement.len() as isize - patch.span.len() as isize;
    }
    offset.saturating_add_signed(shift)
}
