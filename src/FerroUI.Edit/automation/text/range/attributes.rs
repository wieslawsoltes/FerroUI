use super::super::{TextContext, merge_ranges};
use super::super::units::{Style, format_object_boundaries, formatting_runs};
use super::{EditorRange, TextRange, brush_color, check_version};
use crate::text_area::AutomationVisibility;
use crate::{TextDocument, TextEditor};
use ferroui_base::media::{BaselineAlignment, FontStyle, TextDecorationLocation};
use ferroui_controls::automation::provider::{
    ProviderError, TextAttribute as A, TextAttributeRun, TextAttributeSet, TextAttributeValue as V,
    TextProviderError,
};
use ferroui_edit_core::document::{DocumentVersion, SimpleSegment};
use std::ops::{ControlFlow, Range};

const STYLE_ATTRIBUTES: &[A] = &[
    A::FontName, A::FontSize, A::FontWeight, A::FontStyle,
    A::ForegroundColor, A::BackgroundColor, A::UnderlineStyle, A::OverlineStyle,
    A::StrikethroughStyle, A::IsSubscript, A::IsSuperscript,
];

pub(in super::super) fn defaults(editor: &TextEditor) -> TextAttributeSet {
    let properties = editor.default_text_run_properties();
    let style = Style::from_properties(&properties);
    effective_attributes(editor, style, whole_editor_read_only(editor), false)
}

pub(in super::super) fn run(
    context: &TextContext,
    editor: &TextEditor,
    offset: usize,
) -> Result<Option<TextAttributeRun>, ProviderError> {
    let document = editor.document();
    let length = document.text_length();
    if offset > length {
        return Ok(None);
    }
    if offset == length {
        return Ok(Some(TextAttributeRun { range: length..length, attributes: defaults(editor) }));
    }
    let version = document.version();
    let visibility = editor.text_area().automation_visibility(&document)?;
    Ok(local_format(context, editor, &document, &version, &visibility, offset)?.map(|format| format.run))
}

pub(in super::super) fn runs(
    context: &TextContext,
    editor: &TextEditor,
    span: Range<usize>,
) -> Result<Vec<TextAttributeRun>, ProviderError> {
    let document = editor.document();
    if span.start > span.end || span.end > document.text_length() {
        return Err(TextProviderError::InvalidArgument.into());
    }
    let mut runs: Vec<TextAttributeRun> = Vec::new();
    if span.is_empty() {
        return Ok(runs);
    }
    let version = document.version();
    visit_styles(editor, &document, span, false, |range, style| {
        let attributes = style_attributes(editor, &style);
        if let Some(previous) = runs.last_mut().filter(|run| {
            run.range.end == range.start && run.attributes == attributes
        }) {
            previous.range.end = range.end;
        } else {
            runs.push(TextAttributeRun { range, attributes });
        }
        ControlFlow::Continue(())
    })?;
    check_version(&document, version)?;
    context.editor()?;
    Ok(runs)
}

struct LocalFormat {
    run: TextAttributeRun,
    object_at_start: bool,
    object_at_end: bool,
}

pub(in super::super) fn format_unit(
    context: &TextContext,
    editor: &TextEditor,
    document: &TextDocument,
    offset: usize,
) -> Result<Range<usize>, ProviderError> {
    let length = document.text_length();
    if length == 0 {
        return Ok(offset..offset);
    }
    let version = document.version();
    let visibility = editor.text_area().automation_visibility(document)?;
    let read = |position| local_format(context, editor, document, &version, &visibility, position);
    let LocalFormat { run, mut object_at_start, mut object_at_end } =
        read(offset.min(length - 1))?.ok_or(TextProviderError::ElementNotAvailable)?;
    let mut span = run.range;
    while span.start > 0 && !object_at_start {
        let Some(previous) = read(span.start - 1)? else { break };
        if previous.object_at_end || previous.run.attributes != run.attributes {
            break;
        }
        span.start = previous.run.range.start;
        object_at_start = previous.object_at_start;
    }
    while span.end < length && !object_at_end {
        let Some(next) = read(span.end)? else { break };
        if next.object_at_start || next.run.attributes != run.attributes {
            break;
        }
        span.end = next.run.range.end;
        object_at_end = next.object_at_end;
    }
    Ok(span)
}

fn local_format(
    context: &TextContext,
    editor: &TextEditor,
    document: &TextDocument,
    version: &DocumentVersion,
    visibility: &AutomationVisibility,
    offset: usize,
) -> Result<Option<LocalFormat>, ProviderError> {
    let unprojected = visibility.unprojected_at(offset);
    let line = if unprojected.is_none() {
        Some(editor.text_area().automation_line(document, offset)?)
    } else {
        None
    };
    check_version(document, version.clone())?;
    context.editor()?;
    let defaults;
    let (span, style) = if let Some(line) = &line {
        let Some(format) = formatting_runs(line).into_iter().find(|(span, _)| span.contains(&offset)) else {
            return Ok(None);
        };
        format
    } else if let Some(span) = unprojected {
        defaults = editor.default_text_run_properties();
        (span.clone(), Style::from_properties(&defaults))
    } else {
        return Ok(None);
    };
    let (span, read_only) = boolean_run_at(read_only_runs(editor, span.clone()), &span, offset);
    let (mut span, hidden) = boolean_run_at(visibility_runs(&visibility.hidden, span.clone()), &span, offset);
    let mut object_at_start = false;
    let mut object_at_end = false;
    for boundary in line.iter().flat_map(|line| format_object_boundaries(line)) {
        if boundary <= offset && span.start <= boundary {
            span.start = boundary;
            object_at_start = true;
        } else if boundary > offset && boundary <= span.end {
            span.end = boundary;
            object_at_end = true;
        }
    }
    let attributes = effective_attributes(editor, style, read_only, hidden);
    check_version(document, version.clone())?;
    context.editor()?;
    Ok(Some(LocalFormat {
        run: TextAttributeRun { range: span, attributes },
        object_at_start,
        object_at_end,
    }))
}

fn boolean_run_at(runs: Vec<(Range<usize>, V)>, span: &Range<usize>, offset: usize) -> (Range<usize>, bool) {
    runs.into_iter().find(|(range, _)| range.contains(&offset))
        .map_or_else(|| (span.clone(), false), |(range, value)| (range, value == V::Bool(true)))
}

fn effective_attributes(editor: &TextEditor, style: Style<'_>, read_only: bool, hidden: bool) -> TextAttributeSet {
    let mut values = style_attributes(editor, &style);
    values.extend([(A::IsReadOnly, V::Bool(read_only)), (A::IsHidden, V::Bool(hidden))]);
    values
}

fn style_attributes(editor: &TextEditor, style: &Style<'_>) -> TextAttributeSet {
    STYLE_ATTRIBUTES.iter().copied()
        .map(|attribute| (attribute, style_value(editor, style, attribute)))
        .filter(|(_, value)| !matches!(value, V::Mixed | V::NotSupported))
        .collect()
}

pub(super) fn get(range: &EditorRange, attribute: A) -> Result<V, ProviderError> {
    let mut result = None;
    range.visit_attributes(attribute, false, |_, value| {
        if result.as_ref().is_some_and(|current| *current != value) {
            result = Some(V::Mixed);
            ControlFlow::Break(())
        } else {
            result = Some(value);
            ControlFlow::Continue(())
        }
    })?;
    Ok(result.unwrap_or(V::NotSupported))
}

pub(super) fn find(
    range: &EditorRange,
    attribute: A,
    value: &V,
    backwards: bool,
) -> Result<Option<TextRange>, ProviderError> {
    let mut found: Option<Range<usize>> = None;
    let document = range.context.editor()?.document();
    range.visit_attributes(attribute, backwards, |span, current| {
        if current == *value && !matches!(current, V::NotSupported | V::Mixed) {
            found = Some(match found.take() {
                Some(prior) => prior.start.min(span.start)..prior.end.max(span.end),
                None => span,
            });
        } else if found.is_some() {
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    })?;
    found
        .map(|span| range.context.make_range(&document, span))
        .transpose()
}

impl EditorRange {
    fn visit_attributes(
        &self,
        attribute: A,
        backwards: bool,
        mut visit: impl FnMut(Range<usize>, V) -> ControlFlow<()>,
    ) -> Result<(), ProviderError> {
        let (editor, span) = self.state()?;
        let document = editor.document();
        let version = document.version();
        match attribute {
            A::IsReadOnly => {
                let runs = read_only_runs(&editor, span);
                let _ = visit_runs(runs, backwards, &mut visit);
            }
            A::IsHidden => {
                let visibility = editor.text_area().automation_visibility(&document)?;
                let runs = visibility_runs(&visibility.hidden, span);
                let _ = visit_runs(runs, backwards, &mut visit);
            }
            attribute if attribute == A::IsItalic || STYLE_ATTRIBUTES.contains(&attribute) => {
                style_attribute_runs(&editor, &document, span, attribute, backwards, visit)?;
            }
            _ => {}
        }
        check_version(&document, version)?;
        self.context.editor()?;
        Ok(())
    }
}

fn visibility_runs(hidden: &[Range<usize>], span: Range<usize>) -> Vec<(Range<usize>, V)> {
    if span.is_empty() {
        let hidden = hidden
            .iter()
            .any(|fold| fold.start < span.start && span.start < fold.end);
        return vec![(span, V::Bool(hidden))];
    }
    split_boolean_runs(span, hidden.iter().cloned(), true)
}

fn read_only_runs(editor: &TextEditor, span: Range<usize>) -> Vec<(Range<usize>, V)> {
    if whole_editor_read_only(editor) {
        return vec![(span, V::Bool(true))];
    }
    let policy = editor.session().read_only_section_provider();
    if span.is_empty() {
        let readonly = !policy.can_insert(span.start);
        return vec![(span, V::Bool(readonly))];
    }
    let writable = policy
        .get_deletable_segments(SimpleSegment::new(span.start, span.len()))
        .into_iter()
        .map(|segment| segment.offset..segment.end_offset());
    split_boolean_runs(span, writable, false)
}

fn whole_editor_read_only(editor: &TextEditor) -> bool {
    editor.is_read_only() || editor.document().is_read_only() || !editor.is_effectively_enabled()
}

fn split_boolean_runs(
    span: Range<usize>,
    intervals: impl IntoIterator<Item = Range<usize>>,
    inside: bool,
) -> Vec<(Range<usize>, V)> {
    let intervals = merge_ranges(
        intervals
            .into_iter()
            .filter_map(|interval| {
                let range = span.start.max(interval.start)..span.end.min(interval.end);
                (!range.is_empty()).then_some(range)
            })
            .collect(),
    );
    let mut position = span.start;
    let mut runs = Vec::new();
    for range in intervals {
        if position < range.start {
            runs.push((position..range.start, V::Bool(!inside)));
        }
        position = range.end;
        runs.push((range, V::Bool(inside)));
    }
    if position < span.end {
        runs.push((position..span.end, V::Bool(!inside)));
    }
    runs
}

fn style_attribute_runs(
    editor: &TextEditor,
    document: &TextDocument,
    span: Range<usize>,
    attribute: A,
    backwards: bool,
    mut visit: impl FnMut(Range<usize>, V) -> ControlFlow<()>,
) -> Result<(), ProviderError> {
    visit_styles(editor, document, span, backwards, |range, style| {
        visit(range, style_value(editor, &style, attribute))
    })
}

fn visit_styles(
    editor: &TextEditor,
    document: &TextDocument,
    span: Range<usize>,
    backwards: bool,
    mut visit: impl FnMut(Range<usize>, Style<'_>) -> ControlFlow<()>,
) -> Result<(), ProviderError> {
    let area = editor.text_area();
    let visibility = area.automation_visibility(document)?;
    let mut position = if backwards {
        span.end.saturating_sub(1).max(span.start)
    } else {
        span.start
    };
    loop {
        if span.is_empty() && position == document.text_length() {
            let properties = editor.default_text_run_properties();
            let _ = visit(span, Style::from_properties(&properties));
            break;
        }
        let scanned = if let Some(part) = visibility.unprojected_at(position) {
            let properties = editor.default_text_run_properties();
            let clipped = part.start.max(span.start)..part.end.min(span.end);
            if visit(clipped, Style::from_properties(&properties)).is_break()
                || span.is_empty()
            {
                break;
            }
            part.clone()
        } else {
            let line = area.automation_line(document, position)?;
            let runs = super::super::units::style_runs(&line);
            if span.is_empty() {
                let mut runs = runs.peekable();
                while let Some((part, style)) = runs.next() {
                    if part.contains(&position) || runs.peek().is_none() {
                        let _ = visit(span, style);
                        break;
                    }
                }
                break;
            }
            let runs = runs.filter_map(|(part, style)| {
                let clipped = part.start.max(span.start)..part.end.min(span.end);
                (!clipped.is_empty()).then_some((clipped, style))
            }).collect();
            if visit_runs(runs, backwards, &mut visit).is_break() {
                break;
            }
            let last = line.last_document_line();
            line.first_document_line().offset()..last.offset() + last.total_length()
        };
        if backwards && scanned.start > span.start {
            position = scanned.start - 1;
        } else if !backwards && scanned.end < span.end {
            position = scanned.end;
        } else {
            break;
        }
    }
    Ok(())
}

fn visit_runs<T>(
    runs: Vec<(Range<usize>, T)>,
    backwards: bool,
    visit: &mut impl FnMut(Range<usize>, T) -> ControlFlow<()>,
) -> ControlFlow<()> {
    if backwards {
        for (span, value) in runs.into_iter().rev() {
            visit(span, value)?;
        }
    } else {
        for (span, value) in runs {
            visit(span, value)?;
        }
    }
    ControlFlow::Continue(())
}

fn style_value(editor: &TextEditor, style: &Style<'_>, attribute: A) -> V {
    let props = style.properties;
    match attribute {
        A::FontName => V::String(props.typeface().font_family().name().into()),
        A::FontSize => V::Number(props.font_rendering_em_size() * 72.0 / 96.0),
        A::FontWeight => V::Integer(props.typeface().weight().0),
        A::FontStyle => V::String(props.typeface().style().to_string().to_ascii_lowercase()),
        A::IsItalic => V::Bool(props.typeface().style() != FontStyle::Normal),
        A::ForegroundColor => brush_color(style.foreground),
        A::BackgroundColor => brush_color(style.background.or(editor.background().as_ref())),
        A::IsSubscript => V::Bool(props.baseline_alignment() == BaselineAlignment::Subscript),
        A::IsSuperscript => V::Bool(props.baseline_alignment() == BaselineAlignment::Superscript),
        A::UnderlineStyle => decoration_style(style, TextDecorationLocation::Underline),
        A::OverlineStyle => decoration_style(style, TextDecorationLocation::Overline),
        A::StrikethroughStyle => decoration_style(style, TextDecorationLocation::Strikethrough),
        _ => V::NotSupported,
    }
}

fn decoration_style(style: &Style<'_>, location: TextDecorationLocation) -> V {
    let mut lines = style.decorations().filter(|line| line.location() == location);
    let Some(line) = lines.next() else { return V::Integer(0) };
    if lines.next().is_some() || line.stroke_dash_array().is_some_and(|array| !array.list().is_empty()) {
        V::NotSupported
    } else {
        V::Integer(1)
    }
}
