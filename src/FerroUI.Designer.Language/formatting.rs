use crate::{LineIndex, Result, Schema, TextEdit, apply_text_edits, require};
use ferroui_designer_core::{Document, Node, NodeKind, SourceSpan};
use serde::{Deserialize, Serialize};

mod protection;
mod tags;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AttributeLayout {
    #[default]
    Preserve,
    Inline,
    OnePerLine,
    Wrap,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelfClosingSpace {
    #[default]
    Preserve,
    Space,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FormattingOptions {
    pub tab_size: u32,
    pub insert_spaces: bool,
    pub trim_trailing_whitespace: bool,
    pub insert_final_newline: bool,
    pub trim_final_newlines: bool,
    pub attribute_layout: AttributeLayout,
    pub wrap_column: u32,
    pub closing_bracket_on_new_line: bool,
    pub self_closing_space: SelfClosingSpace,
    pub preserve_blank_lines: bool,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: true,
            insert_final_newline: true,
            trim_final_newlines: true,
            attribute_layout: AttributeLayout::Preserve,
            wrap_column: 100,
            closing_bracket_on_new_line: false,
            self_closing_space: SelfClosingSpace::Preserve,
            preserve_blank_lines: false,
        }
    }
}

impl FormattingOptions {
    pub fn validate(&self) -> Result<()> {
        require(
            (1..=16).contains(&self.tab_size),
            "tabSize must be from 1 through 16",
        )?;
        require(
            (20..=240).contains(&self.wrap_column),
            "wrapColumn must be from 20 through 240",
        )
    }
}

pub(crate) fn format(
    document: &Document,
    schema: &Schema,
    options: FormattingOptions,
    selection: Option<SourceSpan>,
) -> Result<Vec<TextEdit>> {
    require(document.is_valid(), "Repair invalid XML before formatting")?;
    options.validate()?;
    if selection.is_some_and(SourceSpan::is_empty) || protection::is_generated(document) {
        return Ok(Vec::new());
    }
    let source = document.source();
    let mut writer = Writer {
        source,
        newline: if source.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        },
        indent: if options.insert_spaces {
            " ".repeat(options.tab_size as usize)
        } else {
            "\t".into()
        },
        options,
        selection,
        disabled: protection::disabled_ranges(document),
        patches: Vec::new(),
    };
    let mut ancestors: Vec<ContentScope> = Vec::new();
    for node in document
        .ast()
        .nodes_in_source_order()
        .filter(|node| node.is_element())
    {
        while ancestors
            .last()
            .is_some_and(|parent| parent.end <= node.span.start)
        {
            ancestors.pop();
        }
        let parent = ancestors.last();
        let preserve_space = match node.attribute("xml:space") {
            Some("preserve") => true,
            Some("default") => false,
            _ => parent.is_some_and(|parent| parent.preserve_space),
        };
        let inside_text = parent.is_some_and(|parent| parent.protect_content);
        let protect_content =
            inside_text || protection::has_significant_content(document, schema, node);
        if !preserve_space && !inside_text {
            writer.opening_tag(node, ancestors.len());
            if !protect_content {
                writer.child_gaps(document, node, ancestors.len());
            }
        }
        ancestors.push(ContentScope {
            end: node.span.end,
            preserve_space,
            protect_content,
        });
    }
    if selection.is_none() {
        writer.document_end(document);
    }
    writer.finish()
}

struct ContentScope {
    end: usize,
    preserve_space: bool,
    protect_content: bool,
}

struct Writer<'a> {
    source: &'a str,
    newline: &'static str,
    indent: String,
    options: FormattingOptions,
    selection: Option<SourceSpan>,
    disabled: Vec<SourceSpan>,
    patches: Vec<(SourceSpan, String)>,
}

impl Writer<'_> {
    fn finish(mut self) -> Result<Vec<TextEdit>> {
        self.patches.sort_by_key(|(span, _)| (span.start, span.end));
        let index = LineIndex::new(self.source);
        let edits = self
            .patches
            .into_iter()
            .map(|(span, new_text)| {
                Ok(TextEdit {
                    range: index.range(span)?,
                    new_text,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if !edits.is_empty() {
            Document::new(apply_text_edits(self.source, &edits)?)?;
        }
        Ok(edits)
    }

    fn replace(&mut self, span: SourceSpan, text: String) {
        let selected = self.selection.is_none_or(|range| {
            if span.is_empty() {
                range.start < span.start && span.start < range.end
            } else {
                range.start <= span.start && span.end <= range.end
            }
        });
        let next = self.disabled.partition_point(|range| {
            range.end < span.start || (range.end == span.start && !span.is_empty())
        });
        let protected = self.disabled.get(next).is_some_and(|guard| {
            if span.is_empty() {
                guard.start < span.start
                    && (span.start < guard.end
                        || (span.start == guard.end && guard.end == self.source.len()))
            } else {
                guard.start < span.end && span.start < guard.end
            }
        });
        if selected && !protected && self.source[span.range()] != text {
            self.patches.push((span, text));
        }
    }

    fn child_gaps(&mut self, document: &Document, node: &Node, depth: usize) {
        let Some(closing) = node.closing_span else {
            return;
        };
        let mut previous = node.opening_span.end;
        let mut changed_container = false;
        for child in node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .filter(|child| child.kind != NodeKind::Text)
        {
            self.line_gap(SourceSpan::new(previous, child.span.start), depth + 1);
            previous = child.span.end;
            changed_container = true;
        }
        if changed_container {
            self.line_gap(SourceSpan::new(previous, closing.start), depth);
        }
    }

    fn line_gap(&mut self, span: SourceSpan, depth: usize) {
        let prior = &self.source[span.range()];
        if !xml_whitespace(prior) {
            return;
        }
        let lines = if self.options.preserve_blank_lines {
            line_breaks(prior).max(1)
        } else {
            1
        };
        self.replace(
            span,
            self.newline.repeat(lines) + &self.indent.repeat(depth),
        );
    }

    fn document_end(&mut self, document: &Document) {
        let last = document
            .ast()
            .roots()
            .iter()
            .rev()
            .filter_map(|id| document.ast().node(*id))
            .find(|node| node.kind != NodeKind::Text)
            .expect("a valid document has an element");
        let tail = &self.source[last.span.end..];
        if !xml_whitespace(tail) {
            return;
        }
        let breaks = line_breaks(tail);
        let mut replacement = if self.options.trim_final_newlines && breaks > 0 {
            self.newline.into()
        } else if self.options.trim_trailing_whitespace {
            self.newline.repeat(breaks)
        } else {
            tail.to_owned()
        };
        if self.options.insert_final_newline && !replacement.ends_with(['\n', '\r']) {
            replacement.push_str(self.newline);
        }
        self.replace(
            SourceSpan::new(last.span.end, self.source.len()),
            replacement,
        );
    }
}

fn line_breaks(text: &str) -> usize {
    text.bytes().filter(|&byte| byte == b'\n').count()
}

fn xml_whitespace(text: &str) -> bool {
    text.bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))
}
