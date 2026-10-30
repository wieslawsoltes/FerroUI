//! Selection and IME are overlays on the document's run properties. Keep one
//! disjoint run partition so the formatter, hit testing and caret see identical
//! fonts, features and advances. This is presentation only: caller ranges never
//! change when preedit is inserted or its provisional selection is removed.
use super::TextStyleOverrides;
use ferroui_base::media::text_formatting::{GenericTextRunProperties, TextRunProperties};
use ferroui_base::media::{IBrush, TextDecorationCollection, TextDecorationLocation, TextDecorations};
use ferroui_base::utilities::ValueSpan;
use std::collections::BTreeSet;
use std::rc::Rc;

type Span = ValueSpan<Rc<dyn TextRunProperties>>;

fn append(spans: &mut Vec<Span>, start: i32, end: i32, properties: Rc<dyn TextRunProperties>) {
    if end <= start { return; }
    if let Some(last) = spans.last_mut() {
        if last.start() + last.length() == start && **last.value() == *properties {
            *last = Span::new(last.start(), end - last.start(), properties);
            return;
        }
    }
    spans.push(Span::new(start, end - start, properties));
}

// O(r log r) normalization, including imported overlaps. Stable first-starting
// precedence matches FormattedTextSource and the design document renderer.
fn partition(defaults: &Rc<dyn TextRunProperties>, styles: Option<&TextStyleOverrides>, length: i32) -> Vec<Span> {
    if length <= 0 { return Vec::new(); }
    let mut ranges: Vec<_> = styles.into_iter().flat_map(|styles| styles.iter())
        .filter_map(|span| {
            let start = span.start().clamp(0, length);
            let end = span.start().saturating_add(span.length()).clamp(0, length);
            (end > start).then(|| Span::new(start, end - start, span.value().clone()))
        }).collect();
    ranges.sort_by_key(Span::start);
    let mut events = vec![(0, false, usize::MAX), (length, false, usize::MAX)];
    for (index, span) in ranges.iter().enumerate() {
        events.push((span.start(), true, index));
        events.push((span.start() + span.length(), false, index));
    }
    events.sort_unstable();
    let mut active = BTreeSet::new();
    let mut output = Vec::with_capacity(ranges.len() + 1);
    let mut previous = 0;
    let mut cursor = 0;
    while cursor < events.len() {
        let at = events[cursor].0;
        let properties = active.first().map(|index: &usize| ranges[*index].value()).unwrap_or(defaults);
        append(&mut output, previous, at, properties.clone());
        while cursor < events.len() && events[cursor].0 == at {
            let (_, begin, index) = events[cursor];
            if index != usize::MAX {
                if begin { active.insert(index); } else { active.remove(&index); }
            }
            cursor += 1;
        }
        previous = at;
    }
    output
}

fn overlay(properties: &Rc<dyn TextRunProperties>, foreground: Option<Rc<dyn IBrush>>, preedit: bool) -> Rc<dyn TextRunProperties> {
    let decorations = if preedit {
        let mut values = properties.text_decorations().map(|items| items.to_vec()).unwrap_or_default();
        if !values.iter().any(|item| item.location() == TextDecorationLocation::Underline) {
            values.extend(TextDecorations::underline().to_vec());
        }
        Some(TextDecorationCollection::from_items(values))
    } else { properties.text_decorations().cloned() };
    Rc::new(GenericTextRunProperties::with_all(
        properties.typeface().clone(), properties.font_rendering_em_size(), decorations,
        foreground.or_else(|| properties.foreground_brush().cloned()),
        properties.background_brush().cloned(), properties.baseline_alignment(),
        properties.culture_info().cloned(), properties.font_features().cloned(),
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn compose(
    defaults: Rc<dyn TextRunProperties>, styles: Option<TextStyleOverrides>, text_length: i32,
    removed: Option<(i32, i32)>, caret: i32, preedit_length: i32,
    selection: Option<(i32, i32, Rc<dyn IBrush>)>,
) -> Option<TextStyleOverrides> {
    if styles.is_none() && preedit_length == 0 && selection.is_none() { return None; }
    let (removed_start, removed_length) = removed.unwrap_or((0, 0));
    let original_length = text_length.saturating_add(removed_length);
    let spans = partition(&defaults, styles.as_ref(), original_length);
    let inherited_at = if removed_length > 0 { removed_start } else { (caret - 1).max(0) };
    let inherited = spans.iter().find(|span| span.start() <= inherited_at && inherited_at < span.start() + span.length())
        .map(|span| span.value().clone()).unwrap_or_else(|| defaults.clone());
    let mut deleted = Vec::with_capacity(spans.len());
    for span in spans {
        let start = span.start();
        let end = start + span.length();
        if removed_length == 0 {
            append(&mut deleted, start, end, span.value().clone());
        } else {
            append(&mut deleted, start, end.min(removed_start), span.value().clone());
            let after = start.max(removed_start + removed_length);
            append(&mut deleted, after - removed_length, end - removed_length, span.value().clone());
        }
    }
    if preedit_length > 0 {
        let mut inserted = Vec::with_capacity(deleted.len() + 2);
        for span in deleted {
            let start = span.start();
            let end = start + span.length();
            append(&mut inserted, start, end.min(caret), span.value().clone());
            append(&mut inserted, start.max(caret) + preedit_length, end + preedit_length, span.value().clone());
        }
        inserted.push(Span::new(caret, preedit_length, overlay(&inherited, None, true)));
        inserted.sort_by_key(Span::start);
        return Some(Rc::from(inserted));
    }
    if let Some((start, length, brush)) = selection {
        let end = start.saturating_add(length);
        let mut selected = Vec::with_capacity(deleted.len() + 2);
        for span in deleted {
            let a = span.start();
            let b = a + span.length();
            append(&mut selected, a, b.min(start), span.value().clone());
            append(&mut selected, a.max(start), b.min(end), overlay(span.value(), Some(brush.clone()), false));
            append(&mut selected, a.max(end), b, span.value().clone());
        }
        return Some(Rc::from(selected));
    }
    (!deleted.is_empty()).then(|| Rc::from(deleted))
}
