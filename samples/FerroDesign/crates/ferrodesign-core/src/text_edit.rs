//! Text editing keeps rich-text ranges in UTF-8 document coordinates.
use crate::{Color, Editor, Node, NodeId, NodeKind, TextDecoration, TextRun};
use std::{collections::BTreeSet, ops::Range};

/// A range edit can leave a property alone, assign an explicit override, or
/// remove the override so that this range follows the layer's base style again.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum TextStyleChange<T> {
    #[default]
    Keep,
    Set(T),
    Inherit,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextStylePatch {
    pub font_family: TextStyleChange<String>,
    pub font_size: TextStyleChange<f64>,
    pub font_weight: TextStyleChange<u16>,
    pub color: TextStyleChange<Color>,
    pub italic: TextStyleChange<bool>,
    pub decoration: TextStyleChange<TextDecoration>,
}

impl TextStylePatch {
    pub fn inherit_all() -> Self {
        Self {
            font_family: TextStyleChange::Inherit,
            font_size: TextStyleChange::Inherit,
            font_weight: TextStyleChange::Inherit,
            color: TextStyleChange::Inherit,
            italic: TextStyleChange::Inherit,
            decoration: TextStyleChange::Inherit,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.font_family, TextStyleChange::Keep)
            && matches!(self.font_size, TextStyleChange::Keep)
            && matches!(self.font_weight, TextStyleChange::Keep)
            && matches!(self.color, TextStyleChange::Keep)
            && matches!(self.italic, TextStyleChange::Keep)
            && matches!(self.decoration, TextStyleChange::Keep)
    }

    fn validate(&self) -> Result<(), TextStyleError> {
        if let TextStyleChange::Set(family) = &self.font_family {
            if family.trim().is_empty() {
                return Err(TextStyleError::InvalidStyle("Font family cannot be empty."));
            }
        }
        if let TextStyleChange::Set(size) = self.font_size {
            if !size.is_finite() || size <= 0.0 {
                return Err(TextStyleError::InvalidStyle("Font size must be a positive finite number."));
            }
        }
        if let TextStyleChange::Set(weight) = self.font_weight {
            if !(1..=1000).contains(&weight) {
                return Err(TextStyleError::InvalidStyle("Font weight must be between 1 and 1000."));
            }
        }
        Ok(())
    }

    fn apply(&self, run: &mut TextRun) {
        apply_change(&mut run.font_family, &self.font_family);
        apply_change(&mut run.font_size, &self.font_size);
        apply_change(&mut run.font_weight, &self.font_weight);
        apply_change(&mut run.color, &self.color);
        apply_change(&mut run.italic, &self.italic);
        apply_change(&mut run.decoration, &self.decoration);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextStyleError {
    MissingNode(NodeId),
    NotText,
    Locked,
    InvalidRange,
    SplitSurrogate,
    InvalidStyle(&'static str),
    InvalidExistingRun,
}

impl std::fmt::Display for TextStyleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingNode(id) => write!(f, "Text layer {id} no longer exists."),
            Self::NotText => f.write_str("Select a text layer to style a range."),
            Self::Locked => f.write_str("Unlock the text layer and its parents before editing."),
            Self::InvalidRange => f.write_str("Select a valid character range in this text layer."),
            Self::SplitSurrogate => f.write_str("A selection boundary splits an emoji or other supplementary character."),
            Self::InvalidStyle(message) => f.write_str(message),
            Self::InvalidExistingRun => f.write_str("An existing text run has an invalid range or style."),
        }
    }
}

impl std::error::Error for TextStyleError {}

/// Convert the native TextBox's (possibly reversed) UTF-16 selection to byte
/// offsets. A half-surrogate boundary is rejected, never rounded or replaced.
pub fn text_range_from_utf16(
    text: &str,
    selection_start: i32,
    selection_end: i32,
) -> Result<Range<usize>, TextStyleError> {
    if selection_start < 0 || selection_end < 0 {
        return Err(TextStyleError::InvalidRange);
    }
    let start = selection_start.min(selection_end) as usize;
    let end = selection_start.max(selection_end) as usize;
    Ok(byte_offset_from_utf16(text, start)?..byte_offset_from_utf16(text, end)?)
}

/// Exact reverse conversion for presenting a document range in a native field.
pub fn text_offset_to_utf16(text: &str, byte_offset: usize) -> Option<i32> {
    if byte_offset > text.len() || !text.is_char_boundary(byte_offset) {
        return None;
    }
    i32::try_from(text[..byte_offset].encode_utf16().count()).ok()
}

fn byte_offset_from_utf16(text: &str, requested: usize) -> Result<usize, TextStyleError> {
    let mut units = 0;
    for (offset, ch) in text.char_indices() {
        if units == requested {
            return Ok(offset);
        }
        units += ch.len_utf16();
        if units > requested {
            return Err(TextStyleError::SplitSurrogate);
        }
    }
    if units == requested { Ok(text.len()) } else { Err(TextStyleError::InvalidRange) }
}

fn apply_change<T: Clone>(value: &mut Option<T>, change: &TextStyleChange<T>) {
    match change {
        TextStyleChange::Keep => {}
        TextStyleChange::Set(next) => *value = Some(next.clone()),
        TextStyleChange::Inherit => *value = None,
    }
}

fn empty_run(start: usize, end: usize) -> TextRun {
    TextRun {
        start, end, font_family: None, font_size: None, font_weight: None,
        color: None, italic: None, decoration: None,
    }
}

fn has_style(run: &TextRun) -> bool {
    run.font_family.is_some() || run.font_size.is_some() || run.font_weight.is_some()
        || run.color.is_some() || run.italic.is_some() || run.decoration.is_some()
}

fn same_style(a: &TextRun, b: &TextRun) -> bool {
    a.font_family == b.font_family && a.font_size == b.font_size
        && a.font_weight == b.font_weight && a.color == b.color
        && a.italic == b.italic && a.decoration == b.decoration
}

fn append_run(runs: &mut Vec<TextRun>, run: TextRun) {
    if !has_style(&run) || run.start >= run.end { return; }
    if let Some(last) = runs.last_mut() {
        if last.end == run.start && same_style(last, &run) {
            last.end = run.end;
            return;
        }
    }
    runs.push(run);
}

impl Editor {
    /// Apply one style patch as one undoable node edit. Validation failures and
    /// already-applied patches do not add history entries or modify selection.
    pub fn style_text_range(
        &mut self,
        id: NodeId,
        range: Range<usize>,
        patch: &TextStylePatch,
    ) -> Result<bool, TextStyleError> {
        let mut current = Some(id);
        while let Some(ancestor) = current {
            let Some(node) = self.document.nodes.get(&ancestor) else { break; };
            if node.locked { return Err(TextStyleError::Locked); }
            current = node.parent;
        }
        self.update_node(id, "Style text range", |node| node.apply_text_style(range, patch))
            .unwrap_or(Err(TextStyleError::MissingNode(id)))
    }
}

impl Node {
    /// Split existing spans at the range boundaries, change only requested
    /// properties, then merge adjacent equal spans. Unstyled portions stay
    /// implicit, so later edits to the layer's defaults still affect them.
    /// Imported overlaps use the renderer's first-starting-span precedence;
    /// tied starts retain their stored order. The result never overlaps.
    pub fn apply_text_style(
        &mut self,
        range: Range<usize>,
        patch: &TextStylePatch,
    ) -> Result<bool, TextStyleError> {
        if self.kind != NodeKind::Text { return Err(TextStyleError::NotText); }
        if range.start > range.end || range.end > self.text.len()
            || !self.text.is_char_boundary(range.start) || !self.text.is_char_boundary(range.end)
        {
            return Err(TextStyleError::InvalidRange);
        }
        patch.validate()?;
        if range.is_empty() || patch.is_empty() { return Ok(false); }
        let mut boundaries = vec![0, self.text.len(), range.start, range.end];
        let mut events = Vec::with_capacity(self.text_runs.len() * 2);
        for (index, run) in self.text_runs.iter().enumerate() {
            if run.start > run.end || run.end > self.text.len()
                || !self.text.is_char_boundary(run.start) || !self.text.is_char_boundary(run.end)
                || run.font_size.is_some_and(|size| !size.is_finite() || size <= 0.0)
                || run.font_weight.is_some_and(|weight| !(1..=1000).contains(&weight))
                || run.font_family.as_ref().is_some_and(|family| family.trim().is_empty())
            { return Err(TextStyleError::InvalidExistingRun); }
            if run.start == run.end { continue; }
            boundaries.extend([run.start, run.end]);
            events.push((run.start, true, index));
            events.push((run.end, false, index));
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        events.sort_unstable(); // End events precede start events at one boundary.
        let mut active = BTreeSet::new();
        let mut event = 0;
        let mut runs = Vec::with_capacity(self.text_runs.len() + 2);
        for boundary in boundaries.windows(2) {
            let (start, end) = (boundary[0], boundary[1]);
            while event < events.len() && events[event].0 <= start {
                let (_, entering, index) = events[event];
                let key = (self.text_runs[index].start, index);
                if entering { active.insert(key); } else { active.remove(&key); }
                event += 1;
            }
            let mut run = active.first()
                .map(|&(_, index)| self.text_runs[index].clone())
                .unwrap_or_else(|| empty_run(start, end));
            run.start = start;
            run.end = end;
            if start >= range.start && end <= range.end { patch.apply(&mut run); }
            append_run(&mut runs, run);
        }
        if self.text_runs == runs { return Ok(false); }
        self.text_runs = runs;
        Ok(true)
    }

    /// Replace a text layer's content while preserving the styles of unchanged
    /// characters. The smallest changed character range inherits the style at
    /// the selection start; typing at a boundary inherits the preceding run.
    /// This also keeps programmatic replacement and non-BMP text on valid byte
    /// boundaries. Native editors should use `replace_text_range_preserving_runs`
    /// when their exact edit coordinates are known: repeated strings make a
    /// common-prefix comparison ambiguous.
    pub fn replace_text_preserving_runs(&mut self, text: String) -> bool {
        if self.kind != NodeKind::Text || self.text == text {
            return false;
        }
        let prefix = self
            .text
            .chars()
            .zip(text.chars())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum::<usize>();
        let suffix = self.text[prefix..]
            .chars()
            .rev()
            .zip(text[prefix..].chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum::<usize>();
        let old_end = self.text.len() - suffix;
        let new_end = text.len() - suffix;
        self.replace_text_range_preserving_runs(text, prefix..old_end, new_end)
            .expect("the inferred replacement has matching UTF-8 boundaries")
    }

    /// Apply an exact replacement using UTF-8 document coordinates. `range`
    /// refers to the old text and `new_end` is the end of the insertion in the
    /// supplied complete new text. The unchanged prefix and suffix must match.
    /// Invalid or stale coordinates leave text and both style collections alone.
    pub fn replace_text_range_preserving_runs(
        &mut self, text: String, range: Range<usize>, new_end: usize,
    ) -> Result<bool, TextStyleError> {
        if self.kind != NodeKind::Text { return Err(TextStyleError::NotText); }
        let prefix = range.start;
        let old_end = range.end;
        if prefix > old_end || old_end > self.text.len() || prefix > new_end || new_end > text.len()
            || !self.text.is_char_boundary(prefix) || !self.text.is_char_boundary(old_end)
            || !text.is_char_boundary(prefix) || !text.is_char_boundary(new_end)
            || self.text[..prefix] != text[..prefix] || self.text[old_end..] != text[new_end..] {
            return Err(TextStyleError::InvalidRange);
        }
        if self.text == text { return Ok(false); }
        let mut runs = Vec::with_capacity(self.text_runs.len() + 2);
        for run in &self.text_runs {
            if run.start >= run.end
                || run.end > self.text.len()
                || !self.text.is_char_boundary(run.start)
                || !self.text.is_char_boundary(run.end)
            {
                continue;
            }
            let mut parts = Vec::with_capacity(3);
            if run.start < prefix {
                push_part(&mut parts, run, run.start, run.end.min(prefix));
            }
            let inherits = if old_end > prefix {
                run.start <= prefix && run.end > prefix
            } else {
                (run.start < prefix && run.end >= prefix) || (prefix == 0 && run.start == 0)
            };
            if inherits {
                push_part(&mut parts, run, prefix, new_end);
            }
            if run.end > old_end {
                let start = run.start.max(old_end) - old_end + new_end;
                let end = run.end - old_end + new_end;
                push_part(&mut parts, run, start, end);
            }
            runs.extend(parts);
        }
        let paragraphs = crate::paragraphs::remap(self, &text, prefix, old_end, new_end);
        self.text = text;
        self.text_runs = runs;
        self.text_paragraphs = paragraphs;
        Ok(true)
    }
}

fn push_part(parts: &mut Vec<TextRun>, template: &TextRun, start: usize, end: usize) {
    if start >= end {
        return;
    }
    if let Some(last) = parts.last_mut() {
        if last.end == start {
            last.end = end;
            return;
        }
    }
    let mut run = template.clone();
    run.start = start;
    run.end = end;
    parts.push(run);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(text: &str, ranges: &[(usize, usize)]) -> Node {
        let mut n = Node::new(NodeKind::Text, "Text", 0.0, 0.0, 100.0, 30.0);
        n.text = text.into();
        n.text_runs = ranges
            .iter()
            .enumerate()
            .map(|(i, &(start, end))| TextRun {
                start,
                end,
                font_family: None,
                font_size: None,
                font_weight: Some(400 + i as u16 * 100),
                color: None,
                italic: None,
                decoration: None,
            })
            .collect();
        n
    }
    fn ranges(n: &Node) -> Vec<(usize, usize)> {
        assert!(n.text_runs.iter().all(|r| r.start < r.end
            && r.end <= n.text.len()
            && n.text.is_char_boundary(r.start)
            && n.text.is_char_boundary(r.end)));
        n.text_runs.iter().map(|r| (r.start, r.end)).collect()
    }
    #[test]
    fn exact_native_edit_retains_style_ownership_in_repeated_paragraphs() {
        let mut n = node("A\nA\n", &[(0, 2), (2, 4)]);
        n.apply_paragraph_style(0..2, &crate::ParagraphStylePatch {
            indent: TextStyleChange::Set(10.0), ..Default::default()
        }).unwrap();
        n.apply_paragraph_style(2..4, &crate::ParagraphStylePatch {
            indent: TextStyleChange::Set(20.0), ..Default::default()
        }).unwrap();
        assert!(n.replace_text_range_preserving_runs("A\nA\nA\n".into(), 0..0, 2).unwrap());
        assert_eq!(ranges(&n), [(0, 4), (4, 6)]);
        assert_eq!(n.text_runs[1].font_weight, Some(500));
        assert_eq!(n.text_paragraphs.iter().map(|style| (style.start, style.end, style.indent)).collect::<Vec<_>>(),
            [(0,2,Some(10.0)), (2,4,Some(10.0)), (4,6,Some(20.0))]);
        crate::validate_text_paragraphs(&n).unwrap();
        assert!(n.replace_text_range_preserving_runs("A\nA\n".into(), 0..2, 0).unwrap());
        assert_eq!(ranges(&n), [(0, 2), (2, 4)]);
        assert_eq!(n.text_paragraphs[1].indent, Some(20.0));
    }

    #[test]
    fn exact_native_replacement_preserves_unicode_suffix_and_rejects_stale_ranges_atomically() {
        let mut n = node("🦀🦀z", &[(0, 4), (4, 8), (8, 9)]);
        assert!(n.replace_text_range_preserving_runs("🦀z".into(), 0..4, 0).unwrap());
        assert_eq!(ranges(&n), [(0, 4), (4, 5)]);
        assert_eq!(n.text_runs[0].font_weight, Some(500), "The second identical glyph survives");
        let before = n.clone();
        for (text, range, new_end) in [
            ("X🦀z", 1..1, 2), ("X🦀z", 0..99, 1),
            ("wrong", 4..5, 5), ("🦀z", 4..4, 1),
        ] {
            assert_eq!(n.replace_text_range_preserving_runs(text.into(), range, new_end), Err(TextStyleError::InvalidRange));
            assert_eq!(n, before);
        }
        assert!(!n.replace_text_range_preserving_runs("🦀z".into(), 0..0, 0).unwrap());
        assert_eq!(n, before);
    }

    #[test]
    fn unicode_insertion_inherits_previous_run_and_shifts_suffix() {
        let mut n = node("A🦀B", &[(0, 5), (5, 6)]);
        n.replace_text_preserving_runs("A🦀éB".into());
        assert_eq!(ranges(&n), [(0, 7), (7, 8)]);
        assert_eq!(n.text_runs[1].font_weight, Some(500));
    }
    #[test]
    fn replacement_preserves_unaffected_styles() {
        let mut n = node("abcDEFghi", &[(0, 3), (3, 6), (6, 9)]);
        n.replace_text_preserving_runs("abc🦀ghi".into());
        assert_eq!(ranges(&n), [(0, 3), (3, 7), (7, 10)]);
    }
    #[test]
    fn deletion_across_runs_leaves_only_surviving_ranges() {
        let mut n = node("abcDEFghi", &[(0, 3), (3, 6), (6, 9)]);
        n.replace_text_preserving_runs("ai".into());
        assert_eq!(ranges(&n), [(0, 1), (1, 2)]);
        assert_eq!(n.text_runs[1].font_weight, Some(600));
    }
    #[test]
    fn combining_character_and_ime_replacement_are_valid() {
        let mut n = node("e\u{301}日本語", &[(0, 12)]);
        n.replace_text_preserving_runs("e\u{301}東京".into());
        assert_eq!(ranges(&n), [(0, 9)]);
    }
    #[test]
    fn entire_replacement_and_empty_text_do_not_leave_invalid_runs() {
        let mut n = node("🦀", &[(0, 4)]);
        n.replace_text_preserving_runs("Hi".into());
        assert_eq!(ranges(&n), [(0, 2)]);
        n.replace_text_preserving_runs(String::new());
        assert!(ranges(&n).is_empty());
    }
    #[test]
    fn suffix_does_not_overlap_common_prefix() {
        let mut n = node("aaaa", &[(0, 4)]);
        n.replace_text_preserving_runs("aaa".into());
        assert_eq!(ranges(&n), [(0, 3)]);
        assert!(!n.replace_text_preserving_runs("aaa".into()));
    }

    #[test]
    fn native_range_conversion_preserves_supplementary_and_combining_characters() {
        let text = "a🦀e\u{301}日";
        assert_eq!(text_range_from_utf16(text, 5, 1), Ok(1..8));
        assert_eq!(text_range_from_utf16(text, 6, 6), Ok(11..11));
        for byte in [0, 1, 5, 6, 8, 11] {
            let native = text_offset_to_utf16(text, byte).unwrap();
            assert_eq!(text_range_from_utf16(text, native, native), Ok(byte..byte));
        }
        assert_eq!(text_range_from_utf16(text, 1, 2), Err(TextStyleError::SplitSurrogate));
        assert_eq!(text_range_from_utf16(text, -1, 3), Err(TextStyleError::InvalidRange));
        assert_eq!(text_range_from_utf16(text, 0, 7), Err(TextStyleError::InvalidRange));
        assert_eq!(text_offset_to_utf16(text, 2), None);
    }

    #[test]
    fn style_patch_splits_and_merges_without_changing_text_or_base_style() {
        let mut n = node("abcdef", &[(0, 6)]);
        n.text_runs[0].color = Some(Color::WHITE);
        let patch = TextStylePatch { font_weight: TextStyleChange::Set(700), ..Default::default() };
        assert_eq!(n.apply_text_style(2..4, &patch), Ok(true));
        assert_eq!(ranges(&n), [(0, 2), (2, 4), (4, 6)]);
        assert_eq!(n.text_runs[1].font_weight, Some(700));
        assert!(n.text_runs.iter().all(|run| run.color == Some(Color::WHITE)));
        assert_eq!(n.text, "abcdef");
        assert_eq!(n.font_weight, Node::default().font_weight);
        assert_eq!(n.apply_text_style(2..4, &patch), Ok(false));
        n.apply_text_style(2..4, &TextStylePatch {
            font_weight: TextStyleChange::Set(400), ..Default::default()
        }).unwrap();
        assert_eq!(ranges(&n), [(0, 6)]);
    }

    #[test]
    fn inheriting_one_property_preserves_other_overrides_and_inheriting_all_creates_a_gap() {
        let mut n = node("abcdef", &[(0, 6)]);
        n.text_runs[0].color = Some(Color::WHITE);
        n.text_runs[0].italic = Some(true);
        n.apply_text_style(1..5, &TextStylePatch {
            color: TextStyleChange::Inherit, ..Default::default()
        }).unwrap();
        assert_eq!(n.text_runs[1].color, None);
        assert_eq!(n.text_runs[1].font_weight, Some(400));
        assert_eq!(n.text_runs[1].italic, Some(true));
        n.apply_text_style(2..4, &TextStylePatch::inherit_all()).unwrap();
        assert_eq!(ranges(&n), [(0, 1), (1, 2), (4, 5), (5, 6)]);
        n.apply_text_style(0..6, &TextStylePatch::inherit_all()).unwrap();
        assert!(n.text_runs.is_empty());
    }

    #[test]
    fn gap_styling_and_explicit_false_remain_independent_of_layer_defaults() {
        let mut n = node("a🦀b", &[(0, 1), (5, 6)]);
        n.italic = true;
        let range = text_range_from_utf16(&n.text, 1, 3).unwrap();
        n.apply_text_style(range, &TextStylePatch {
            italic: TextStyleChange::Set(false),
            decoration: TextStyleChange::Set(TextDecoration::None),
            ..Default::default()
        }).unwrap();
        assert_eq!(ranges(&n), [(0, 1), (1, 5), (5, 6)]);
        assert_eq!(n.text_runs[1].italic, Some(false));
        assert_eq!(n.text_runs[1].font_weight, None);
        assert_eq!(n.text_runs[1].decoration, Some(TextDecoration::None));
    }

    #[test]
    fn imported_overlaps_normalize_using_native_first_starting_span_precedence() {
        let mut n = node("abcdefgh", &[(2, 8), (0, 5), (0, 3)]);
        n.apply_text_style(1..6, &TextStylePatch {
            italic: TextStyleChange::Set(true), ..Default::default()
        }).unwrap();
        assert_eq!(ranges(&n), [(0, 1), (1, 5), (5, 6), (6, 8)]);
        assert_eq!(n.text_runs[0].font_weight, Some(500));
        assert_eq!(n.text_runs[1].font_weight, Some(500));
        assert_eq!(n.text_runs[2].font_weight, Some(400));
        assert_eq!(n.text_runs[3].font_weight, Some(400));
    }

    #[test]
    fn invalid_style_and_byte_ranges_are_atomic_and_empty_selection_is_a_noop() {
        let mut n = node("a🦀b", &[(0, 6)]);
        let original = n.clone();
        let patch = TextStylePatch { font_size: TextStyleChange::Set(32.0), ..Default::default() };
        assert_eq!(n.apply_text_style(2..5, &patch), Err(TextStyleError::InvalidRange));
        assert_eq!(n.apply_text_style(4..1, &patch), Err(TextStyleError::InvalidRange));
        assert_eq!(n.apply_text_style(1..1, &patch), Ok(false));
        for size in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
            assert!(n.apply_text_style(0..6, &TextStylePatch {
                font_size: TextStyleChange::Set(size), ..Default::default()
            }).is_err());
        }
        assert!(n.apply_text_style(0..6, &TextStylePatch {
            font_weight: TextStyleChange::Set(1001), ..Default::default()
        }).is_err());
        assert!(n.apply_text_style(0..6, &TextStylePatch {
            font_family: TextStyleChange::Set("  ".into()), ..Default::default()
        }).is_err());
        assert_eq!(n, original);
    }

    #[test]
    fn range_style_history_is_one_entry_with_exact_undo_redo_and_noop_coalescing() {
        let mut doc = crate::Document::new("Text styles");
        let id = doc.insert_node(doc.pages[0].id, node("a🦀b", &[(0, 6)])).unwrap();
        let before = doc.nodes[&id].text_runs.clone();
        let mut editor = Editor::new(doc);
        editor.selection = vec![id];
        let patch = TextStylePatch {
            font_size: TextStyleChange::Set(28.0),
            color: TextStyleChange::Set(Color::WHITE),
            ..Default::default()
        };
        assert_eq!(editor.style_text_range(id, 1..5, &patch), Ok(true));
        let after = editor.document.nodes[&id].text_runs.clone();
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.style_text_range(id, 1..5, &patch), Ok(false));
        assert!(editor.style_text_range(id, 2..5, &patch).is_err());
        assert_eq!(editor.history_len(), 1);
        assert!(editor.undo());
        assert_eq!(editor.document.nodes[&id].text_runs, before);
        assert_eq!(editor.selection, [id]);
        assert!(editor.redo());
        assert_eq!(editor.document.nodes[&id].text_runs, after);
        editor.document.validate().unwrap();
    }

    #[test]
    fn style_edit_respects_locked_ancestors_and_rejects_non_text_layers() {
        let mut doc = crate::Document::new("Locked text");
        let parent = doc.insert_node(doc.pages[0].id,
            Node::new(NodeKind::Frame, "Parent", 0.0, 0.0, 100.0, 100.0)).unwrap();
        let id = doc.insert_node(parent, node("abc", &[])).unwrap();
        doc.nodes.get_mut(&parent).unwrap().locked = true;
        let mut editor = Editor::new(doc);
        let patch = TextStylePatch { italic: TextStyleChange::Set(true), ..Default::default() };
        assert_eq!(editor.style_text_range(id, 0..3, &patch), Err(TextStyleError::Locked));
        editor.document.nodes.get_mut(&parent).unwrap().locked = false;
        assert_eq!(editor.style_text_range(parent, 0..0, &patch), Err(TextStyleError::NotText));
        assert_eq!(editor.style_text_range(u64::MAX, 0..0, &patch), Err(TextStyleError::MissingNode(u64::MAX)));
        assert_eq!(editor.history_len(), 0);
    }

    #[test]
    fn restyling_many_adjacent_spans_keeps_serialized_storage_bounded() {
        let mut n = node(&"x".repeat(5000), &[]);
        n.text_runs = (0..5000).map(|index| {
            let mut run = empty_run(index, index + 1);
            run.font_weight = Some(if index % 2 == 0 { 400 } else { 700 });
            run
        }).collect();
        n.apply_text_style(0..5000, &TextStylePatch {
            font_weight: TextStyleChange::Set(500), ..Default::default()
        }).unwrap();
        assert_eq!(ranges(&n), [(0, 5000)]);
        for _ in 0..20 {
            assert_eq!(n.apply_text_style(0..5000, &TextStylePatch {
                font_weight: TextStyleChange::Set(500), ..Default::default()
            }), Ok(false));
        }
    }
}
