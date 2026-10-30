//! Source-backed model tests for WinUI text highlighting. See documents/NOTICE.md.

use super::*;
use crate::test_support::test_scope;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Brushes, Color, IBrush, LinearGradientBrush, SolidColorBrush};
use ferroui_base::metadata::{MarkupTyped, from_markup_value, into_markup_value};
use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

fn brush(id: u8) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(id, 0, 0)))
}
fn brush_id(value: &Option<Rc<dyn IBrush>>) -> Option<u8> {
    value
        .as_ref()
        .and_then(|brush| brush.as_solid_color_brush())
        .map(|brush| brush.color().r)
}
fn layer(range: Range<usize>, id: u8) -> TextHighlightRegion {
    TextHighlightRegion::new(range, Some(brush(id * 2 - 1)), Some(brush(id * 2)))
}

#[test]
fn text_highlight_merge_matches_all_source_overlap_cases() {
    // Literal fixtures from TextHighlightMergeUnitTests.cpp. Endpoints in these
    // tuples remain inclusive to make comparison with the pinned source direct.
    type Fixture = (
        &'static str,
        Vec<(usize, usize, u8)>,
        Vec<(usize, usize, u8)>,
    );
    let fixtures: Vec<Fixture> = vec![
        (
            "NonOverlapped",
            vec![(0, 1, 1), (5, 6, 2)],
            vec![(0, 1, 1), (5, 6, 2)],
        ),
        (
            "OverlapNextPartial",
            vec![(5, 10, 1), (3, 8, 2)],
            vec![(3, 8, 2), (9, 10, 1)],
        ),
        (
            "OverlapPrevPartial",
            vec![(5, 10, 1), (8, 12, 2)],
            vec![(5, 7, 1), (8, 12, 2)],
        ),
        (
            "OverlapNextFull",
            vec![(5, 10, 1), (3, 12, 2)],
            vec![(3, 12, 2)],
        ),
        (
            "OverlapEqualFull",
            vec![(5, 10, 1), (5, 12, 2)],
            vec![(5, 12, 2)],
        ),
        (
            "OverlapEqualPartial",
            vec![(5, 10, 1), (5, 8, 2)],
            vec![(5, 8, 2), (9, 10, 1)],
        ),
        (
            "OverlapEqualMultiple",
            vec![(5, 10, 1), (11, 20, 2), (21, 30, 3), (5, 25, 4)],
            vec![(5, 25, 4), (26, 30, 3)],
        ),
        (
            "OverlapInner",
            vec![(0, 10, 1), (4, 8, 2)],
            vec![(0, 3, 1), (4, 8, 2), (9, 10, 1)],
        ),
        (
            "Overlap1",
            vec![(0, 2, 1), (4, 6, 2), (2, 4, 3)],
            vec![(0, 1, 1), (2, 4, 3), (5, 6, 2)],
        ),
        (
            "OverlapMultiple",
            vec![(0, 5, 1), (8, 12, 2), (14, 20, 3), (20, 30, 4), (2, 25, 5)],
            vec![(0, 1, 1), (2, 25, 5), (26, 30, 4)],
        ),
    ];
    for (name, input, expected) in fixtures {
        let result = TextHighlightMerge::from_regions(
            input
                .into_iter()
                .map(|(start, end, id)| layer(start..end + 1, id)),
        );
        let actual: Vec<_> = result
            .iter()
            .map(|region| {
                let foreground = brush_id(&region.foreground).unwrap();
                let background = brush_id(&region.background).unwrap();
                assert_eq!(foreground + 1, background, "{name}");
                (region.range.start, region.range.end - 1, background / 2)
            })
            .collect();
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn text_highlight_merge_agrees_with_character_layering_and_handles_extreme_endpoints() {
    let mut merge = TextHighlightMerge::new();
    let mut expected = [None; 64];
    for id in 1..=64u8 {
        let a = (usize::from(id) * 17) % 65;
        let b = (usize::from(id) * 29 + 3) % 65;
        let range = a.min(b)..a.max(b);
        for value in &mut expected[range.clone()] {
            *value = Some(id * 2);
        }
        merge.add_region(layer(range, id));
        let mut actual = [None; 64];
        let mut previous_end = 0;
        for region in merge.iter() {
            assert!(region.range.start >= previous_end);
            assert!(!region.range.is_empty());
            previous_end = region.range.end;
            for value in &mut actual[region.range.clone()] {
                *value = brush_id(&region.background);
            }
        }
        assert_eq!(actual, expected, "after layer {id}");
    }

    let mut extreme = TextHighlightMerge::from_regions([layer(0..usize::MAX, 1)]);
    extreme.add_region(layer(usize::MAX - 2..usize::MAX, 2));
    extreme.add_region(layer(usize::MAX..usize::MAX, 3));
    extreme.add_region(layer(Range { start: 10, end: 0 }, 4));
    let output = extreme.into_regions();
    assert_eq!(output.len(), 2);
    assert_eq!(output[0].range, 0..usize::MAX - 2);
    assert_eq!(output[1].range, usize::MAX - 2..usize::MAX);
    assert_eq!(brush_id(&output[1].background), Some(4));
}

#[test]
fn text_highlight_ranges_preserve_native_value_semantics_and_validate_half_open_conversion() {
    let original = TextHighlightRange::new(2, 4);
    let list = TextRangeCollection::from_items([original.clone()]);
    original.set_start_index(99);
    let returned = list.get(0).unwrap();
    returned.set_length(99);
    let snapshot = list.snapshot();
    snapshot[0].set_start_index(88);
    list.for_each(|range| range.set_length(88));
    assert_eq!(list.get(0).unwrap().to_range(), Ok(2..6));
    assert_eq!(list.clone(), list);

    assert_eq!(
        TextHighlightRange::from_range(3..7).unwrap().to_string(),
        "3,4"
    );
    assert_eq!(
        TextHighlightRange::parse(" 3, 4 ").unwrap().range(),
        Some(3..7)
    );
    assert!(TextHighlightRange::parse("3,4,5").is_err());
    assert!(TextHighlightRange::parse("3,").is_err());
    assert!(TextHighlightRange::parse("2147483648,1").is_err());
    assert_eq!(
        TextHighlightRange::new(-1, 1).to_range(),
        Err(TextHighlightError::NegativeRange)
    );
    assert_eq!(TextHighlightRange::new(1, -1).range(), None);
    assert_eq!(
        TextHighlightRange::new(i32::MAX, 1).to_range(),
        Err(TextHighlightError::RangeOverflow)
    );
    assert_eq!(TextHighlightRange::new(3, 0).range(), Some(3..3));
    assert_eq!(
        TextHighlightRange::from_range(Range { start: 2, end: 1 }),
        Err(TextHighlightError::ReversedRange)
    );
    assert_eq!(
        TextHighlightRange::from_range(0..usize::MAX),
        Err(TextHighlightError::RangeOverflow)
    );

    list.insert(usize::MAX, TextHighlightRange::new(7, 1));
    assert_eq!(list.count(), 2);
    assert_eq!(list.get(1).unwrap().range(), Some(7..8));
    assert_eq!(list.get(2), None);
    assert!(!list.set(usize::MAX, TextHighlightRange::default()));
    assert_eq!(list.remove_at(usize::MAX), None);
    assert_eq!(list.remove_at(1).unwrap().range(), Some(7..8));
    let before = list.snapshot();
    assert!(list.add_span(Range { start: 9, end: 2 }).is_err());
    assert_eq!(list.snapshot(), before);
}

#[test]
fn text_highlight_range_markup_writes_a_box_then_copies_the_collection_value() {
    let _scope = test_scope();
    let metadata = <TextHighlightRange as MarkupTyped>::MARKUP;
    assert_eq!(metadata.name, "TextRange");
    let value = (metadata.constructors[0].invoke)(&[]).unwrap();
    let start = metadata.find_property("StartIndex").unwrap();
    let length = metadata.find_property("Length").unwrap();
    (start.set.unwrap())(&[value.clone(), into_markup_value(4i32)]).unwrap();
    (length.set.unwrap())(&[value.clone(), into_markup_value(6i32)]).unwrap();
    let boxed = from_markup_value::<Rc<TextHighlightRange>>(&value).unwrap();
    assert_eq!(boxed.range(), Some(4..10));

    let ranges = TextRangeCollection::new();
    let collection = <TextRangeCollection as MarkupTyped>::MARKUP;
    let add = collection
        .methods
        .iter()
        .find(|method| method.name == "Add")
        .unwrap();
    (add.invoke)(&[into_markup_value(ranges.clone()), value]).unwrap();
    boxed.set_start_index(44);
    assert_eq!(ranges.get(0).unwrap().range(), Some(4..10));
}

#[test]
fn text_highlighter_layers_use_defaults_and_clip_invalid_ranges() {
    let _scope = test_scope();
    let first = TextHighlighter::new();
    first.set_foreground(Some(brush(1))).unwrap();
    first.set_background(Some(brush(2))).unwrap();
    first.ranges().add(TextHighlightRange::new(0, 30));
    let later = TextHighlighter::new();
    later.set_background(Some(brush(4))).unwrap();
    later.ranges().add(TextHighlightRange::new(2, 3));
    later.ranges().add(TextHighlightRange::new(-1, 3));
    later
        .ranges()
        .add(TextHighlightRange::new(i32::MAX, i32::MAX));
    later.ranges().add(TextHighlightRange::new(7, 0));
    later.ranges().add(TextHighlightRange::new(30, 4));
    let collection = TextHighlighterCollection::from_items([first, later]);
    let regions = collection.merged_regions(10, Some(brush(9)), Some(brush(10)));
    assert_eq!(
        regions.iter().map(|r| r.range.clone()).collect::<Vec<_>>(),
        [0..2, 2..5, 5..10]
    );
    assert_eq!(brush_id(&regions[1].foreground), Some(9));
    assert_eq!(brush_id(&regions[1].background), Some(4));

    let high = TextHighlighter::new();
    high.ranges().add(TextHighlightRange::new(1, 1));
    high.set_background(Some(brush(1))).unwrap();
    let low = TextHighlighter::new();
    low.ranges().add(TextHighlightRange::new(2, 1));
    low.set_background(Some(brush(2))).unwrap();
    let layers = TextHighlighterCollection::from_items([high, low]);
    // In "a😀b", UTF-16 offsets 1 and 3 bound the scalar.
    let regions = layers.merged_regions_with(4, None, None, |range| {
        let start = if range.start == 2 { 1 } else { range.start };
        let end = if range.end == 2 { 3 } else { range.end };
        start..end
    });
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].range, 1..3);
    assert_eq!(brush_id(&regions[0].background), Some(2));
}

#[test]
fn text_highlighter_collection_observes_items_and_detaches_removed_subscriptions() {
    let _scope = test_scope();
    let collection = TextHighlighterCollection::new();
    let changes = Rc::new(Cell::new(0));
    let changes_copy = changes.clone();
    let subscription = collection.invalidated(move || changes_copy.set(changes_copy.get() + 1));
    let item = TextHighlighter::new();
    collection.add(item.clone());
    collection.add(item.clone());
    assert_eq!(changes.get(), 2);
    item.ranges().add(TextHighlightRange::new(1, 2));
    assert_eq!(
        changes.get(),
        3,
        "one subscription for duplicate identities"
    );
    item.set_foreground(Some(Brushes::black())).unwrap();
    assert_eq!(changes.get(), 4);
    collection.remove(item.clone());
    assert_eq!(changes.get(), 5);
    item.ranges().clear();
    assert_eq!(
        changes.get(),
        6,
        "the remaining occurrence stays subscribed"
    );
    collection.remove(item.clone());
    assert_eq!(changes.get(), 7);
    item.ranges().add(TextHighlightRange::new(1, 2));
    assert_eq!(changes.get(), 7);

    collection.add(item.clone());
    let replacement = TextHighlighter::new();
    assert!(collection.set(0, replacement.clone()));
    let before = changes.get();
    item.set_foreground(None).unwrap();
    assert_eq!(changes.get(), before);
    replacement.ranges().add(TextHighlightRange::new(0, 1));
    assert_eq!(changes.get(), before + 1);
    collection.clear();
    let before = changes.get();
    replacement.ranges().clear();
    assert_eq!(changes.get(), before);
    subscription.dispose();
    collection.add(item);
    assert_eq!(changes.get(), before);
}

#[test]
fn text_highlighter_mutable_brush_changes_invalidate_and_replacement_unsubscribes() {
    let _scope = test_scope();
    let highlighter = TextHighlighter::new();
    let mutable = SolidColorBrush::with_color(Color::from_rgb(1, 2, 3));
    highlighter
        .set_foreground(Some(mutable.clone().into()))
        .unwrap();
    highlighter
        .set_background(Some(mutable.clone().into()))
        .unwrap();
    let changes = Rc::new(Cell::new(0));
    let capture = changes.clone();
    let _subscription = highlighter.invalidated(move || capture.set(capture.get() + 1));
    mutable.set_color(Color::from_rgb(4, 5, 6));
    assert_eq!(
        changes.get(),
        1,
        "one subscription for the same foreground/background brush"
    );
    let prior_foreground = highlighter.foreground();
    let bad: Rc<dyn IBrush> = LinearGradientBrush::new().into();
    assert_eq!(
        highlighter.try_set_foreground(Some(bad)),
        Err(TextHighlightError::NonSolidColorBrush)
    );
    assert!(highlighter.foreground() == prior_foreground);
    assert_eq!(changes.get(), 1, "failed assignment is atomic");
    highlighter.set_foreground(None).unwrap();
    highlighter.set_background(None).unwrap();
    let before = changes.get();
    mutable.set_color(Color::from_rgb(7, 8, 9));
    assert_eq!(changes.get(), before);
}

#[test]
fn text_highlighter_observer_ownership_is_weak_and_callbacks_can_reenter() {
    let _scope = test_scope();
    let collection = TextHighlighterCollection::new();
    let weak_collection = collection.downgrade();
    let first = TextHighlighter::new();
    let weak_first = first.downgrade();
    let ranges = first.ranges();
    let mutable = SolidColorBrush::new();
    first.set_background(Some(mutable.clone().into())).unwrap();
    collection.add(first.clone());
    let once = Rc::new(Cell::new(true));
    let weak = weak_collection.clone();
    let _subscription = collection.invalidated(move || {
        if once.replace(false) {
            if let Some(collection) = weak.upgrade() {
                collection.add(TextHighlighter::new());
            }
        }
    });
    ranges.add(TextHighlightRange::new(0, 1));
    assert_eq!(
        collection.count(),
        2,
        "callbacks can reenter after every collection borrow ends"
    );
    let _item_subscription = first.invalidated(|| {});
    drop(collection);
    assert!(weak_collection.upgrade().is_none());
    drop(first);
    assert!(
        weak_first.upgrade().is_none(),
        "range/brush observers do not retain the highlighter"
    );
    ranges.add(TextHighlightRange::new(2, 1));
    mutable.set_color(Color::from_rgb(1, 2, 3));
}
