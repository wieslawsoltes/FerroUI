//! Independent regressions for paragraph tabs over the real framework formatter.
use super::testing::{
    format_line, format_lines, run_properties, CustomDrawableRun, DrawCall, ListTextSource,
    RecordingDrawingSink, SingleBufferTextSource, TextTestScope,
};
use super::{
    GenericTextParagraphProperties, GenericTextRunProperties, InterWordJustification, TabAlignment,
    TabLeader, TextCharacters, TextFormatter, TextParagraphProperties, TextRunCache,
    TextRunProperties, TextTabError, TextTabStop, TextTabStops,
};
use crate::media::{
    BaselineAlignment, Brushes, CharacterHit, FlowDirection, TextAlignment, TextWrapping, Typeface,
};
use crate::utilities::{CultureInfo, NumberFormatInfo};
use crate::Point;
use std::rc::Rc;

fn tabs(
    properties: Rc<dyn TextRunProperties>,
    stops: &[TextTabStop],
    direction: FlowDirection,
    wrapping: TextWrapping,
    origin: f64,
) -> Rc<dyn TextParagraphProperties> {
    Rc::new(
        GenericTextParagraphProperties::with_all(
            direction,
            TextAlignment::Start,
            true,
            false,
            properties,
            wrapping,
            0.0,
            0.0,
            0.0,
        )
        .with_tabs(
            TextTabStops::new(stops.iter().copied()).unwrap(),
            48.0,
            origin,
        ),
    )
}
fn stop(position: f64, alignment: TabAlignment) -> TextTabStop {
    TextTabStop::new(position, alignment, TabLeader::Spaces)
}
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-7, "{actual} != {expected}");
}
fn colored_properties(culture: Option<CultureInfo>) -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        12.0,
        None,
        Some(Brushes::black()),
        None,
        BaselineAlignment::Baseline,
        culture,
        None,
    ))
}

#[test]
fn paragraph_tabs_advance_to_positions_instead_of_adding_fixed_widths() {
    let _scope = TextTestScope::new();
    let props = run_properties(12.0);
    let source = SingleBufferTextSource::new("a\tb\tc", props.clone());
    let line = format_line(
        &source,
        0,
        200.0,
        &tabs(
            props,
            &[],
            FlowDirection::LeftToRight,
            TextWrapping::NoWrap,
            0.0,
        ),
        None,
    )
    .unwrap();
    close(
        line.get_distance_from_character_hit(CharacterHit::new(2)),
        48.0,
    );
    close(
        line.get_distance_from_character_hit(CharacterHit::new(4)),
        96.0,
    );
    close(line.width(), 102.0);
}

#[test]
fn paragraph_tabs_align_left_center_right_and_decimal_fields() {
    let _scope = TextTestScope::new();
    for (alignment, start, end) in [
        (TabAlignment::Left, 60.0, 84.0),
        (TabAlignment::Center, 48.0, 72.0),
        (TabAlignment::Right, 36.0, 60.0),
        (TabAlignment::Decimal, 48.0, 72.0),
    ] {
        let props = run_properties(12.0);
        let source = SingleBufferTextSource::new("a\t12.5", props.clone());
        let line = format_line(
            &source,
            0,
            200.0,
            &tabs(
                props,
                &[stop(60.0, alignment)],
                FlowDirection::LeftToRight,
                TextWrapping::NoWrap,
                0.0,
            ),
            None,
        )
        .unwrap();
        close(
            line.get_distance_from_character_hit(CharacterHit::new(2)),
            start,
        );
        close(line.width(), end);
    }
}

#[test]
fn paragraph_tabs_decimal_uses_run_culture_and_crosses_style_boundaries() {
    let _scope = TextTestScope::new();
    // Base runs without a culture-data provider. Supply the number conventions
    // explicitly; a culture name alone intentionally uses invariant separators.
    let culture = CultureInfo::get_culture_info("pl-PL")
        .with_number_format(NumberFormatInfo::new().with_number_decimal_separator(","));
    let props = colored_properties(Some(culture.clone()));
    let decimal_props = Rc::new(GenericTextRunProperties::with_all(
        Typeface::default_typeface(),
        12.0,
        None,
        Some(Brushes::blue()),
        None,
        BaselineAlignment::Baseline,
        Some(culture),
        None,
    ));
    let source = ListTextSource::new(vec![
        Rc::new(TextCharacters::from_str("a\t12", props.clone())),
        Rc::new(TextCharacters::from_str(",5", decimal_props)),
    ]);
    let line = format_line(
        &source,
        0,
        200.0,
        &tabs(
            props,
            &[stop(60.0, TabAlignment::Decimal)],
            FlowDirection::LeftToRight,
            TextWrapping::NoWrap,
            0.0,
        ),
        None,
    )
    .unwrap();
    close(
        line.get_distance_from_character_hit(CharacterHit::new(4)),
        60.0,
    );
}

#[test]
fn paragraph_tabs_rtl_decimal_uses_visual_field_coordinate() {
    let _scope = TextTestScope::new();
    let props = run_properties(12.0);
    let source = SingleBufferTextSource::new("אב\t12.5", props.clone());
    let line = format_line(
        &source,
        0,
        200.0,
        &tabs(
            props,
            &[stop(60.0, TabAlignment::Decimal)],
            FlowDirection::RightToLeft,
            TextWrapping::NoWrap,
            0.0,
        ),
        None,
    )
    .unwrap();
    close(
        line.get_distance_from_character_hit(CharacterHit::new(5)),
        140.0,
    );
    assert_eq!(line.length(), 7);
}

#[test]
fn paragraph_tabs_rtl_alignment_is_physical_and_stops_measure_from_right_edge() {
    let _scope = TextTestScope::new();
    for (alignment, left, right) in [
        (TabAlignment::Left, 140.0, 158.0),
        (TabAlignment::Center, 131.0, 149.0),
        (TabAlignment::Right, 122.0, 140.0),
    ] {
        let props = run_properties(12.0);
        let source = SingleBufferTextSource::new("אב\t123", props.clone());
        let line = format_line(
            &source,
            0,
            200.0,
            &tabs(
                props,
                &[stop(60.0, alignment)],
                FlowDirection::RightToLeft,
                TextWrapping::NoWrap,
                0.0,
            ),
            None,
        )
        .unwrap();
        close(
            line.get_distance_from_character_hit(CharacterHit::new(3)),
            left,
        );
        close(
            line.get_distance_from_character_hit(CharacterHit::with_trailing_length(5, 1)),
            right,
        );
    }
}

#[test]
fn paragraph_tabs_account_for_indent_and_ignore_stops_beyond_available_page() {
    let _scope = TextTestScope::new();
    let props = run_properties(12.0);
    let source = SingleBufferTextSource::new("a\tb", props.clone());
    let line = format_line(
        &source,
        0,
        100.0,
        &tabs(
            props.clone(),
            &[stop(60.0, TabAlignment::Left)],
            FlowDirection::LeftToRight,
            TextWrapping::NoWrap,
            14.0,
        ),
        None,
    )
    .unwrap();
    close(
        line.get_distance_from_character_hit(CharacterHit::new(2)),
        46.0,
    );
    let line = format_line(
        &source,
        0,
        80.0,
        &tabs(
            props,
            &[stop(100.0, TabAlignment::Left)],
            FlowDirection::LeftToRight,
            TextWrapping::NoWrap,
            0.0,
        ),
        None,
    )
    .unwrap();
    close(
        line.get_distance_from_character_hit(CharacterHit::new(2)),
        48.0,
    );
}

#[test]
fn paragraph_tabs_preserve_shaped_clusters_in_cross_run_fields() {
    let scope = TextTestScope::new();
    scope.shaper().add_ligature("ffi");
    let props = run_properties(12.0);
    let source = ListTextSource::new(vec![
        Rc::new(TextCharacters::from_str("a\t", props.clone())),
        Rc::new(TextCharacters::from_str("ffi", props.clone())),
        Rc::new(TextCharacters::from_str("x", run_properties(24.0))),
    ]);
    let line = format_line(
        &source,
        0,
        200.0,
        &tabs(
            props,
            &[stop(60.0, TabAlignment::Right)],
            FlowDirection::LeftToRight,
            TextWrapping::NoWrap,
            0.0,
        ),
        None,
    )
    .unwrap();
    close(
        line.get_distance_from_character_hit(CharacterHit::new(2)),
        42.0,
    );
    close(line.width(), 60.0);
    assert_eq!(line.length(), 6);
}

#[test]
fn paragraph_tabs_measure_embedded_objects_in_the_aligned_field() {
    let _scope = TextTestScope::new();
    let props = run_properties(12.0);
    let source = ListTextSource::new(vec![
        Rc::new(TextCharacters::from_str("a\t", props.clone())),
        Rc::new(CustomDrawableRun::new(props.clone())),
        Rc::new(TextCharacters::from_str("b", props.clone())),
    ]);
    let line = format_line(
        &source,
        0,
        200.0,
        &tabs(
            props,
            &[stop(60.0, TabAlignment::Right)],
            FlowDirection::LeftToRight,
            TextWrapping::NoWrap,
            0.0,
        ),
        None,
    )
    .unwrap();
    close(
        line.get_distance_from_character_hit(CharacterHit::new(2)),
        40.0,
    );
    close(
        line.get_distance_from_character_hit(CharacterHit::new(3)),
        54.0,
    );
    close(line.width(), 60.0);
    assert_eq!(line.length(), 4);
}

#[test]
fn paragraph_tabs_wrapping_recomputes_each_line_and_preserves_all_text_positions() {
    let _scope = TextTestScope::new();
    for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
        for alignment in [
            TabAlignment::Left,
            TabAlignment::Center,
            TabAlignment::Right,
            TabAlignment::Decimal,
        ] {
            let props = run_properties(12.0);
            let text = "a\t123456.78 long field\tword";
            let source = SingleBufferTextSource::new(text, props.clone());
            let lines = format_lines(
                &source,
                text.len() as i32,
                60.0,
                &tabs(
                    props,
                    &[stop(48.0, alignment)],
                    direction,
                    TextWrapping::Wrap,
                    0.0,
                ),
            );
            assert!(lines.len() > 1);
            assert_eq!(
                lines.iter().map(|line| line.length()).sum::<i32>(),
                text.len() as i32
            );
            for line in lines {
                assert!(line.length() > 0);
                assert!(
                    line.width() <= 60.00001,
                    "{direction:?} {alignment:?}: {}",
                    line.width()
                );
            }
        }
    }
}

#[test]
fn paragraph_tabs_reflow_does_not_mutate_cached_shaping_or_retained_lines() {
    let scope = TextTestScope::new();
    let props = run_properties(12.0);
    let source = SingleBufferTextSource::new("a\tb", props.clone());
    let properties = tabs(
        props,
        &[stop(80.0, TabAlignment::Left)],
        FlowDirection::LeftToRight,
        TextWrapping::NoWrap,
        0.0,
    );
    let formatter = <dyn TextFormatter>::current();
    let cache = TextRunCache::new();
    let first = formatter
        .format_line_with_cache(&source, 0, 100.0, &properties, None, Some(&cache))
        .unwrap();
    let calls = scope.shaper().shape_call_count();
    let second = formatter
        .format_line_with_cache(&source, 0, 60.0, &properties, None, Some(&cache))
        .unwrap();
    assert_eq!(scope.shaper().shape_call_count(), calls);
    close(first.width(), 86.0);
    close(second.width(), 54.0);
    close(
        first.get_distance_from_character_hit(CharacterHit::new(2)),
        80.0,
    );
}

#[test]
fn paragraph_tabs_leaders_and_bars_draw_with_framework_sink_without_extra_text() {
    let _scope = TextTestScope::new();
    for leader in [
        TabLeader::Spaces,
        TabLeader::Dots,
        TabLeader::Dashes,
        TabLeader::Lines,
        TabLeader::ThickLines,
        TabLeader::Equals,
    ] {
        let props = colored_properties(None);
        let source = SingleBufferTextSource::new("a\tb", props.clone());
        let line = format_line(
            &source,
            0,
            200.0,
            &tabs(
                props,
                &[
                    TextTabStop::new(60.0, TabAlignment::Left, leader),
                    stop(100.0, TabAlignment::Bar),
                ],
                FlowDirection::LeftToRight,
                TextWrapping::NoWrap,
                0.0,
            ),
            None,
        )
        .unwrap();
        let mut sink = RecordingDrawingSink::new();
        line.draw(&mut sink, Point::new(0.0, 0.0));
        let lines = sink
            .calls()
            .iter()
            .filter(|call| matches!(call, DrawCall::Line(..)))
            .count();
        assert_eq!(
            lines,
            if leader == TabLeader::Spaces {
                1
            } else if leader == TabLeader::Equals {
                3
            } else {
                2
            }
        );
        assert_eq!(line.length(), 3);
        close(line.width(), 66.0);
        assert_eq!(sink.transform_depth(), 0);
    }
}

#[test]
fn paragraph_tabs_bar_appears_without_a_tab_character_and_does_not_change_width() {
    let _scope = TextTestScope::new();
    for (direction, x) in [
        (FlowDirection::LeftToRight, 60.0),
        (FlowDirection::RightToLeft, 140.0),
    ] {
        let props = colored_properties(None);
        let source = SingleBufferTextSource::new("abc", props.clone());
        let line = format_line(
            &source,
            0,
            200.0,
            &tabs(
                props,
                &[stop(60.0, TabAlignment::Bar)],
                direction,
                TextWrapping::NoWrap,
                0.0,
            ),
            None,
        )
        .unwrap();
        close(line.width(), 18.0);
        let mut sink = RecordingDrawingSink::new();
        line.draw(&mut sink, Point::new(0.0, 0.0));
        assert!(sink
            .calls()
            .iter()
            .any(|call| matches!(call, DrawCall::Line(p1, p2) if p1.x == x && p2.x == x)));
    }
}

#[test]
fn paragraph_tabs_value_parser_and_clone_keep_validated_immutable_configuration() {
    let original = TextTabStops::parse("96,Decimal,Dots;48,Left").unwrap();
    assert_eq!(original.as_slice()[0].position, 48.0);
    let replacement = original
        .with_tab(TextTabStop::new(
            96.0,
            TabAlignment::Right,
            TabLeader::Dashes,
        ))
        .unwrap();
    assert_eq!(original.as_slice()[1].alignment, TabAlignment::Decimal);
    assert_eq!(replacement.as_slice()[1].alignment, TabAlignment::Right);
    assert!(TextTabStops::parse("NaN,Left").is_err());
    assert_eq!(
        TextTabStops::parse("1,Middle"),
        Err(TextTabError::InvalidSyntax)
    );
    assert!(TextTabStops::parse("-1,Left").is_err());
}

#[test]
fn paragraph_tabs_justification_keeps_stops_and_leaders_while_filling_the_last_leading_field() {
    let _scope = TextTestScope::new();
    for (direction, alignment, text, field_start) in [
        (
            FlowDirection::LeftToRight,
            TabAlignment::Left,
            "a b\tc d e",
            60.0,
        ),
        (
            FlowDirection::RightToLeft,
            TabAlignment::Right,
            "א ב\tד ה ו",
            140.0,
        ),
    ] {
        let props = colored_properties(None);
        let source = SingleBufferTextSource::new(text, props.clone());
        let line = format_line(
            &source,
            0,
            200.0,
            &tabs(
                props,
                &[TextTabStop::new(60.0, alignment, TabLeader::Dots)],
                direction,
                TextWrapping::NoWrap,
                0.0,
            ),
            None,
        )
        .unwrap();
        let first_word_end = line.get_distance_from_character_hit(CharacterHit::new(1));
        line.justify(&InterWordJustification::new(120.0));
        close(line.width(), 120.0);
        close(
            line.get_distance_from_character_hit(CharacterHit::new(1)),
            first_word_end,
        );
        close(
            line.get_distance_from_character_hit(CharacterHit::new(4)),
            field_start,
        );
        let mut sink = RecordingDrawingSink::new();
        line.draw(&mut sink, Point::new(0.0, 0.0));
        assert_eq!(
            sink.calls()
                .iter()
                .filter(|call| matches!(call, DrawCall::Line(..)))
                .count(),
            1
        );
    }
}

#[test]
fn paragraph_tabs_justification_does_not_move_centered_trailing_or_decimal_anchors() {
    let _scope = TextTestScope::new();
    for alignment in [
        TabAlignment::Center,
        TabAlignment::Right,
        TabAlignment::Decimal,
    ] {
        let props = run_properties(12.0);
        let source = SingleBufferTextSource::new("a b\t1 2.5", props.clone());
        let line = format_line(
            &source,
            0,
            200.0,
            &tabs(
                props,
                &[stop(60.0, alignment)],
                FlowDirection::LeftToRight,
                TextWrapping::NoWrap,
                0.0,
            ),
            None,
        )
        .unwrap();
        let width = line.width();
        let positions: Vec<_> = (0..line.length())
            .map(|index| line.get_distance_from_character_hit(CharacterHit::new(index)))
            .collect();
        line.justify(&InterWordJustification::new(200.0));
        close(line.width(), width);
        for (index, before) in positions.into_iter().enumerate() {
            close(
                line.get_distance_from_character_hit(CharacterHit::new(index as i32)),
                before,
            );
        }
    }
}
