use super::{
    Bold, CharacterFormat, DocumentLinkField, Hyperlink, HyperlinkBehavior, Inline,
    InlineCollection, InlineUIContainer, Italic, LineBreak, Paragraph, ParagraphFormat, Run, Span,
    Underline, UnderlineStyle,
};
use crate::Control;
use ferroui_base::Ref;
use std::collections::HashSet;
use std::ops::Range;

/// An owned image of document content. Character data and formatting collections
/// are independent of the live document; embedded controls retain their identity.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentSnapshot {
    pub paragraphs: Vec<ParagraphSnapshot>,
    /// Instruction metadata in visible UTF-16 coordinates. Rendered hyperlink
    /// segments carry the matching identity; one field may cross paragraphs.
    pub link_fields: Vec<DocumentLinkField>,
    /// The next available identity in this snapshot. A live document also keeps
    /// a monotonic allocation floor across undo, so retired handles never alias.
    pub next_link_field_id: u64,
}
impl Default for DocumentSnapshot {
    fn default() -> Self {
        Self {
            paragraphs: Vec::new(),
            link_fields: Vec::new(),
            next_link_field_id: 1,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphSnapshot {
    pub format: ParagraphFormat,
    pub inlines: Vec<InlineSnapshot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InlineSnapshot {
    pub format: CharacterFormat,
    pub kind: InlineSnapshotKind,
}

/// The structured content of an inline snapshot. Containers retain their type so
/// a formatting edit or an undo does not flatten the document's content tree.
#[derive(Clone, Debug, PartialEq)]
pub enum InlineSnapshotKind {
    Run(String),
    Span(Vec<InlineSnapshot>),
    Bold(Vec<InlineSnapshot>),
    Italic(Vec<InlineSnapshot>),
    Underline(Vec<InlineSnapshot>),
    Hyperlink {
        inlines: Vec<InlineSnapshot>,
        field_id: Option<u64>,
        behavior: HyperlinkBehavior,
        navigate_uri: Option<String>,
        target_name: Option<String>,
        is_enabled: bool,
        is_tab_stop: bool,
        tab_index: i32,
        is_visited: bool,
        underline_style: UnderlineStyle,
    },
    LineBreak,
    Embedded(Option<Ref<Control>>),
}

impl DocumentSnapshot {
    /// Creates plain content, normalizing CRLF and CR paragraph separators to LF.
    pub fn from_text(text: &str) -> Self {
        let text = normalize_paragraph_breaks(text);
        Self {
            paragraphs: text
                .split('\n')
                .map(|text| ParagraphSnapshot {
                    format: ParagraphFormat::default(),
                    inlines: if text.is_empty() {
                        Vec::new()
                    } else {
                        vec![InlineSnapshot::run(text)]
                    },
                })
                .collect(),
            ..Default::default()
        }
    }

    pub fn text(&self) -> String {
        let mut text = String::new();
        for (index, paragraph) in self.paragraphs.iter().enumerate() {
            if index != 0 {
                text.push('\n');
            }
            for inline in &paragraph.inlines {
                inline.append_text(&mut text);
            }
        }
        text
    }

    pub fn text_length(&self) -> usize {
        self.paragraphs
            .iter()
            .map(ParagraphSnapshot::text_length)
            .sum::<usize>()
            + self.paragraphs.len().saturating_sub(1)
    }

    /// Checks all values before a restore or insertion can mutate a live tree.
    pub fn validate(&self) -> Result<(), super::DocumentError> {
        let mut controls = HashSet::new();
        let mut stack = Vec::new();
        for paragraph in &self.paragraphs {
            paragraph.format.validate()?;
            for inline in &paragraph.inlines {
                stack.push((inline, 0usize));
            }
        }
        while let Some((inline, depth)) = stack.pop() {
            if depth > 256 {
                return Err(super::DocumentError::ContentTooDeep);
            }
            inline.format.validate()?;
            if let InlineSnapshotKind::Embedded(Some(child)) = &inline.kind {
                if !controls.insert(&**child as *const Control as usize) {
                    return Err(super::DocumentError::DuplicateEmbeddedControl);
                }
            }
            if let Some(children) = inline.children() {
                for child in children {
                    stack.push((child, depth + 1));
                }
            }
        }
        self.validate_link_fields()?;
        Ok(())
    }

    pub(crate) fn locate(&self, offset: usize) -> (usize, usize) {
        let mut remaining = offset;
        for (index, paragraph) in self.paragraphs.iter().enumerate() {
            let length = paragraph.text_length();
            if remaining <= length || index + 1 == self.paragraphs.len() {
                return (index, remaining.min(length));
            }
            remaining -= length + 1;
        }
        (0, 0)
    }

    pub(crate) fn slice_unchecked(&self, range: Range<usize>) -> Self {
        if self.paragraphs.is_empty() {
            return Self {
                next_link_field_id: self.next_link_field_id,
                ..Self::default()
            };
        }
        let (start_index, start_offset) = self.locate(range.start);
        let (end_index, end_offset) = self.locate(range.end);
        let mut paragraphs = self.paragraphs[start_index..=end_index].to_vec();
        let last = paragraphs.len() - 1;
        if start_index == end_index {
            paragraphs[0].inlines = slice_inlines(&paragraphs[0].inlines, start_offset..end_offset);
        } else {
            paragraphs[0].inlines = slice_inlines(
                &paragraphs[0].inlines,
                start_offset..paragraphs[0].text_length(),
            );
            paragraphs[last].inlines = slice_inlines(&paragraphs[last].inlines, 0..end_offset);
        }
        let link_fields = self
            .link_fields
            .iter()
            .filter_map(|field| {
                let start = field.range.start.max(range.start);
                let end = field.range.end.min(range.end);
                (start < end).then(|| {
                    let mut field = field.clone();
                    field.range = start - range.start..end - range.start;
                    field
                })
            })
            .collect();
        let mut result = Self {
            paragraphs,
            link_fields,
            next_link_field_id: self.next_link_field_id,
        };
        result.prune_orphan_field_projections();
        result
    }
}

impl ParagraphSnapshot {
    pub fn from_paragraph(paragraph: &Paragraph) -> Self {
        Self {
            format: ParagraphFormat::from_paragraph(paragraph),
            inlines: capture_inlines(&paragraph.inlines()),
        }
    }

    pub fn text(&self) -> String {
        let mut text = String::new();
        for inline in &self.inlines {
            inline.append_text(&mut text);
        }
        text
    }

    pub fn text_length(&self) -> usize {
        self.inlines.iter().map(InlineSnapshot::text_length).sum()
    }
}

impl InlineSnapshot {
    pub fn run(text: impl Into<String>) -> Self {
        Self {
            format: CharacterFormat::default(),
            kind: InlineSnapshotKind::Run(text.into()),
        }
    }

    pub fn from_inline(inline: &Ref<Inline>) -> Self {
        let kind = if let Some(run) = inline.cast::<Run>() {
            InlineSnapshotKind::Run(run.text().unwrap_or_default())
        } else if let Some(link) = inline.cast::<Hyperlink>() {
            InlineSnapshotKind::Hyperlink {
                inlines: capture_inlines(&link.inlines()),
                field_id: link.field_id(),
                behavior: link.snapshot_behavior(),
                navigate_uri: link.navigate_uri(),
                target_name: link.target_name(),
                is_enabled: link.is_enabled(),
                is_tab_stop: link.is_tab_stop(),
                tab_index: link.tab_index(),
                is_visited: link.is_visited(),
                underline_style: link.underline_style(),
            }
        } else if let Some(span) = inline.cast::<Span>() {
            let inlines = capture_inlines(&span.inlines());
            if inline.is::<Bold>() {
                InlineSnapshotKind::Bold(inlines)
            } else if inline.is::<Italic>() {
                InlineSnapshotKind::Italic(inlines)
            } else if inline.is::<Underline>() {
                InlineSnapshotKind::Underline(inlines)
            } else {
                InlineSnapshotKind::Span(inlines)
            }
        } else if inline.is::<LineBreak>() {
            InlineSnapshotKind::LineBreak
        } else if let Some(container) = inline.cast::<InlineUIContainer>() {
            InlineSnapshotKind::Embedded(container.child())
        } else {
            let mut text = String::new();
            inline.append_text(&mut text);
            InlineSnapshotKind::Run(text)
        };
        Self {
            format: CharacterFormat::from_element(inline),
            kind,
        }
    }

    pub fn text_length(&self) -> usize {
        match &self.kind {
            InlineSnapshotKind::Run(text) => text.encode_utf16().count(),
            InlineSnapshotKind::LineBreak => inline_line_break().len(),
            InlineSnapshotKind::Embedded(child) => usize::from(child.is_some()),
            _ => self
                .children()
                .map(|children| children.iter().map(Self::text_length).sum())
                .unwrap_or(0),
        }
    }

    pub fn text(&self) -> String {
        let mut text = String::new();
        self.append_text(&mut text);
        text
    }

    pub fn children(&self) -> Option<&[InlineSnapshot]> {
        match &self.kind {
            InlineSnapshotKind::Span(children)
            | InlineSnapshotKind::Bold(children)
            | InlineSnapshotKind::Italic(children)
            | InlineSnapshotKind::Underline(children)
            | InlineSnapshotKind::Hyperlink {
                inlines: children, ..
            } => Some(children),
            _ => None,
        }
    }

    pub(crate) fn children_mut(&mut self) -> Option<&mut Vec<InlineSnapshot>> {
        match &mut self.kind {
            InlineSnapshotKind::Span(children)
            | InlineSnapshotKind::Bold(children)
            | InlineSnapshotKind::Italic(children)
            | InlineSnapshotKind::Underline(children)
            | InlineSnapshotKind::Hyperlink {
                inlines: children, ..
            } => Some(children),
            _ => None,
        }
    }

    pub(crate) fn append_text(&self, text: &mut String) {
        match &self.kind {
            InlineSnapshotKind::Run(value) => text.push_str(value),
            InlineSnapshotKind::LineBreak => text.push_str(inline_line_break()),
            InlineSnapshotKind::Embedded(Some(_)) => text.push('\u{fffc}'),
            InlineSnapshotKind::Embedded(None) => {}
            _ => {
                if let Some(children) = self.children() {
                    for child in children {
                        child.append_text(text);
                    }
                }
            }
        }
    }

    /// Returns a format-preserving scalar-aligned slice of the inline.
    pub(crate) fn slice(&self, range: Range<usize>) -> Option<Self> {
        let length = self.text_length();
        if range.start >= range.end {
            return None;
        }
        if range.start == 0 && range.end >= length {
            return Some(self.clone());
        }
        let mut result = self.clone();
        match &self.kind {
            InlineSnapshotKind::Run(text) => {
                let start = utf16_to_byte(text, range.start)?;
                let end = utf16_to_byte(text, range.end)?;
                result.kind = InlineSnapshotKind::Run(text[start..end].to_owned());
            }
            InlineSnapshotKind::LineBreak => {
                let text = inline_line_break();
                let start = utf16_to_byte(text, range.start)?;
                let end = utf16_to_byte(text, range.end)?;
                result.kind = InlineSnapshotKind::Run(text[start..end].to_owned());
            }
            InlineSnapshotKind::Embedded(_) => return None,
            _ => {
                let inlines = slice_inlines(self.children()?, range);
                *result.children_mut()? = inlines;
            }
        }
        Some(result)
    }
}

pub(crate) fn capture_inlines(inlines: &InlineCollection) -> Vec<InlineSnapshot> {
    inlines
        .snapshot()
        .iter()
        .map(InlineSnapshot::from_inline)
        .collect()
}

pub(crate) fn slice_inlines(
    inlines: &[InlineSnapshot],
    range: Range<usize>,
) -> Vec<InlineSnapshot> {
    let mut result = Vec::new();
    let mut offset = 0;
    for inline in inlines {
        let end = offset + inline.text_length();
        if end > range.start && offset < range.end {
            if let Some(part) =
                inline.slice(range.start.saturating_sub(offset)..range.end.min(end) - offset)
            {
                result.push(part);
            }
        } else if end == offset && range.start <= offset && offset < range.end {
            result.push(inline.clone());
        }
        offset = end;
    }
    result
}

pub(crate) fn utf16_to_byte(text: &str, offset: usize) -> Option<usize> {
    let mut position = 0;
    for (byte, scalar) in text.char_indices() {
        if position == offset {
            return Some(byte);
        }
        position += scalar.len_utf16();
        if position > offset {
            return None;
        }
    }
    (position == offset).then_some(text.len())
}

pub(crate) fn normalize_paragraph_breaks(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// A soft line break of the model: one unit, U+000B, on every platform (decision D10).
fn inline_line_break() -> &'static str {
    super::line_break::RICH_LINE_BREAK
}
