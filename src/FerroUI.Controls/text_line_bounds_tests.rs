//! `TextBlock.TextLineBounds`, a FerroUI addition after WinUI's `TextBlock.TextLineBounds`
//! (docs/porting/DEVIATIONS.md, Text block): how the box of each line is bounded. The reference
//! has no such property; these tests are specific to the port.
//!
//! The test fonts (`ferroui_base::media::text_formatting::testing`) have an ascent of 0.8 em, a
//! descent of 0.2 em, a line gap of 0.1 em and no cap height of their own, so it is seven tenths
//! of the ascent (0.56 em): at the default size of 12 a line is 13.2 high, the cap height 6.72.

use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control, TextBlock};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{TextLineBounds, TextWrapping};
use ferroui_base::Ref;

const LINE: f64 = 13.2;
const CAP: f64 = 6.72;
const ASCENT: f64 = 9.6;
const DESCENT: f64 = 2.4;
const HALF_GAP: f64 = 0.6;

fn text_block(text: &str, bounds: TextLineBounds) -> Ref<TextBlock> {
    let target = TextBlock::new();
    target.set_text(Some(text));
    target.set_horizontal_alignment(HorizontalAlignment::Left);
    target.set_vertical_alignment(VerticalAlignment::Top);
    target.set_use_layout_rounding(false);
    target.set_text_line_bounds(bounds);
    target
}

fn measured_height(target: &Ref<TextBlock>) -> f64 {
    let _root = TestRoot::with_child(target.clone().upcast::<Control>());
    _root.execute_initial_layout_pass();
    target.desired_size().height
}

fn assert_near(expected: f64, actual: f64, what: &str) {
    assert!((expected - actual).abs() < 1e-9, "{what}: expected {expected}, got {actual}");
}

#[test]
fn full_is_the_default_and_is_inherited() {
    let _scope = test_scope();
    let target = TextBlock::new();
    assert_eq!(TextLineBounds::Full, target.text_line_bounds());

    // A content presenter (or any control) sets it for the text blocks below it.
    let border = Border::new();
    border.set_child(Some(target.clone().upcast()));
    let _root = TestRoot::with_child(border.clone());
    TextBlock::set_text_line_bounds_on(&border, TextLineBounds::Tight);
    assert_eq!(TextLineBounds::Tight, target.text_line_bounds());
    assert_eq!(TextLineBounds::Tight, TextBlock::get_text_line_bounds(&border));

    // A value of the text block wins over the inherited one.
    target.set_text_line_bounds(TextLineBounds::Full);
    assert_eq!(TextLineBounds::Full, target.text_line_bounds());
}

#[test]
fn the_height_of_a_line_follows_its_bounds() {
    let _scope = test_scope();
    let cases = [
        (TextLineBounds::Full, LINE),
        (TextLineBounds::TrimToCapHeight, CAP + DESCENT + HALF_GAP),
        (TextLineBounds::TrimToBaseline, HALF_GAP + ASCENT),
        (TextLineBounds::Tight, CAP),
    ];
    for (bounds, height) in cases {
        let target = text_block("Hxg", bounds);
        assert_near(height, measured_height(&target), &format!("{bounds:?}"));
        assert_near(height, target.text_layout().height(), &format!("the layout of {bounds:?}"));
    }
}

#[test]
fn every_line_is_trimmed() {
    let _scope = test_scope();
    let target = text_block("Hxg\nHxg", TextLineBounds::Tight);
    target.set_text_wrapping(TextWrapping::Wrap);
    assert_near(2.0 * CAP, measured_height(&target), "two tight lines");
}

#[test]
fn a_change_measures_the_text_again() {
    let _scope = test_scope();
    let target = text_block("Hxg", TextLineBounds::Full);
    let root = TestRoot::with_child(target.clone().upcast::<Control>());
    root.execute_initial_layout_pass();
    assert_near(LINE, target.desired_size().height, "full");

    target.set_text_line_bounds(TextLineBounds::Tight);
    assert!(!target.is_measure_valid());
    target.update_layout();
    assert_near(CAP, target.desired_size().height, "tight");
}

#[test]
fn a_text_block_with_trimmed_lines_does_not_clip_its_ink() {
    let _scope = test_scope();
    let target = TextBlock::new();
    // The default of the reference.
    assert!(target.clip_to_bounds());

    for bounds in [TextLineBounds::TrimToCapHeight, TextLineBounds::TrimToBaseline, TextLineBounds::Tight] {
        target.set_text_line_bounds(bounds);
        assert!(!target.clip_to_bounds(), "{bounds:?}");
    }

    target.set_text_line_bounds(TextLineBounds::Full);
    assert!(target.clip_to_bounds());

    // An inherited value counts too, and a text block that does not clip stays so.
    let border = Border::new();
    border.set_child(Some(target.clone().upcast()));
    let _root = TestRoot::with_child(border.clone());
    TextBlock::set_text_line_bounds_on(&border, TextLineBounds::Tight);
    target.clear_value(TextBlock::text_line_bounds_property());
    assert!(!target.clip_to_bounds());
    target.set_clip_to_bounds(false);
    TextBlock::set_text_line_bounds_on(&border, TextLineBounds::Full);
    assert!(!target.clip_to_bounds());
}
