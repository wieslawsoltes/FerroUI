//! Direct tests for `TextFormatterImpl::split_text_runs`. Each branch is
//! exercised in isolation with synthetic run stubs — independent of the wrap
//! algorithm that's its main caller. Many of these scenarios are unreachable
//! through the wrap path on its own (the wrap loop carefully avoids requesting
//! splits inside non-splittable runs), but the function is also called by the
//! collapsing properties and the ellipsis types, which have weaker invariants.
//!
//! Key invariants under test (must hold regardless of split position):
//!   * Sum of run lengths is preserved (no content lost or duplicated).
//!   * The reported first length equals the sum of lengths in first.

use std::rc::Rc;

use crate::media::text_formatting::formatting_object_pool::FormattingObjectPool;
use crate::media::text_formatting::testing::{
    format_line, paragraph_properties, run_properties, SingleBufferTextSource, TextTestScope,
};
use crate::media::text_formatting::text_run::{text_run_any, TextRun};
use crate::media::text_formatting::{ShapedTextRun, TextFormatterImpl};
use crate::media::TextWrapping;

/// Minimal concrete run for tests — neither a shaped run nor a drawable run
/// from the consumer perspective. Behaves like an atomic, non-splittable run
/// with a configurable length.
struct TestStubRun {
    length: i32,
}

impl TextRun for TestStubRun {
    fn length(&self) -> i32 {
        self.length
    }

    text_run_any!();
}

fn stub(length: i32) -> Rc<dyn TextRun> {
    Rc::new(TestStubRun { length })
}

type Runs = Option<Vec<Rc<dyn TextRun>>>;

fn split(runs: &[Rc<dyn TextRun>], length: i32) -> (Runs, Runs, i32) {
    let (split, first_length) =
        TextFormatterImpl::split_text_runs_with_length(runs, length, FormattingObjectPool::instance());

    let (first, second) = split.deconstruct();

    (first, second, first_length)
}

fn give_back(first: Runs, second: Runs) {
    let pool = FormattingObjectPool::instance();

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
    pool.verify_all_returned();
}

fn total(runs: &Runs) -> i32 {
    runs.as_ref().map_or(0, |runs| runs.iter().map(|run| run.length()).sum())
}

fn same(x: &Rc<dyn TextRun>, y: &Rc<dyn TextRun>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(x), Rc::as_ptr(y))
}

/// Asserts the central invariant: sum of (first|second) run lengths
/// equals the input total, and `first_length` matches the
/// sum of first's run lengths.
fn assert_content_preserved(input: &[Rc<dyn TextRun>], first: &Runs, second: &Runs, first_length: i32) {
    let input_total: i32 = input.iter().map(|run| run.length()).sum();

    assert_eq!(input_total, total(first) + total(second));
    assert_eq!(total(first), first_length);
}

#[test]
fn split_inside_non_shaped_run_does_not_drop_run() {
    let runs = [stub(3)];

    let (first, second, first_length) = split(&runs, 1);

    assert_content_preserved(&runs, &first, &second, first_length);

    give_back(first, second);
}

#[test]
fn split_length_zero_returns_null_first_and_all_in_second() {
    let runs = [stub(2), stub(3)];

    let (first, second, first_length) = split(&runs, 0);

    assert!(first.is_none());
    assert_eq!(second.as_ref().unwrap().len(), 2);
    assert_eq!(first_length, 0);
    assert_eq!(total(&second), 5);

    give_back(first, second);
}

#[test]
fn split_length_equals_total_puts_all_in_first() {
    let runs = [stub(2), stub(3)];

    let (first, second, first_length) = split(&runs, 5);

    assert_eq!(first.as_ref().unwrap().len(), 2);
    assert!(second.is_none());
    assert_eq!(first_length, 5);
    assert_content_preserved(&runs, &first, &second, first_length);

    give_back(first, second);
}

#[test]
fn split_length_past_total_puts_all_in_first() {
    let runs = [stub(2)];

    let (first, second, first_length) = split(&runs, 99);

    assert_eq!(first.as_ref().unwrap().len(), 1);
    assert!(second.is_none());
    assert_eq!(first_length, 2);

    give_back(first, second);
}

#[test]
fn split_at_boundary_between_two_runs_goes_to_first_or_second_cleanly() {
    let runs = [stub(2), stub(3)];

    // length=2 means "everything up to and including the first run on first".
    let (first, second, first_length) = split(&runs, 2);

    assert_eq!(first.as_ref().unwrap().len(), 1);
    assert!(same(&runs[0], &first.as_ref().unwrap()[0]));
    assert_eq!(second.as_ref().unwrap().len(), 1);
    assert!(same(&runs[1], &second.as_ref().unwrap()[0]));
    assert_eq!(first_length, 2);
    assert_content_preserved(&runs, &first, &second, first_length);

    give_back(first, second);
}

#[test]
fn split_before_drawable_that_does_not_fit_puts_drawable_in_second() {
    // [text(2), drawable(1), text(2)] split at length=2.
    let runs = [stub(2), stub(1), stub(2)];

    let (first, second, first_length) = split(&runs, 2);

    assert_eq!(first_length, 2);
    assert_content_preserved(&runs, &first, &second, first_length);
    assert_eq!(first.as_ref().unwrap().len(), 1);
    assert_eq!(second.as_ref().unwrap().len(), 2);
    assert!(same(&runs[1], &second.as_ref().unwrap()[0])); // drawable at start of second

    give_back(first, second);
}

#[test]
fn split_strictly_inside_non_shaped_run_snaps_before_it() {
    // [text(2), drawable(3), text(2)] split at length=3 — strictly inside
    // the drawable. The drawable is atomic, so the split must snap to a
    // boundary: BEFORE the drawable, so the first length is shorter than
    // requested but content is preserved.
    let runs = [stub(2), stub(3), stub(2)];

    let (first, second, first_length) = split(&runs, 3);

    assert_content_preserved(&runs, &first, &second, first_length);
    assert_eq!(first_length, 2);
    assert!(same(&runs[1], &second.as_ref().unwrap()[0]));

    give_back(first, second);
}

#[test]
fn split_strictly_inside_non_shaped_run_at_start_of_list_overflows() {
    // [drawable(5)] split at length=2 — the drawable is the first run, has
    // no content before it, and is bigger than the requested length. The
    // contract here is to overflow the drawable into first.
    let runs = [stub(5)];

    let (first, second, first_length) = split(&runs, 2);

    assert_content_preserved(&runs, &first, &second, first_length);
    assert_eq!(first_length, 5); // overflow
    assert_eq!(first.as_ref().unwrap().len(), 1);
    assert!(same(&runs[0], &first.as_ref().unwrap()[0]));

    give_back(first, second);
}

#[test]
fn split_at_boundary_before_drawable_mid_list() {
    let runs = [stub(3), stub(2), stub(3)];

    let (first, second, first_length) = split(&runs, 3);

    assert_eq!(first_length, 3);
    assert_content_preserved(&runs, &first, &second, first_length);

    give_back(first, second);
}

#[test]
fn split_at_boundary_after_drawable_mid_list() {
    let runs = [stub(3), stub(2), stub(3)];

    let (first, second, first_length) = split(&runs, 5);

    assert_eq!(first_length, 5);
    assert_content_preserved(&runs, &first, &second, first_length);
    assert_eq!(first.as_ref().unwrap().len(), 2);
    assert_eq!(second.as_ref().unwrap().len(), 1);

    give_back(first, second);
}

#[test]
fn split_with_zero_length_run_inside_does_not_drop_it() {
    // Zero-length runs appear after shaped runs. Splitting at the boundary
    // should keep the zero-length run somewhere — not silently discard it.
    let runs = [stub(2), stub(0), stub(2)];

    let (first, second, first_length) = split(&runs, 2);

    assert_eq!(first_length, 2);

    // The zero-length run still has identity — assert it's in exactly one half.
    let all_runs: Vec<&Rc<dyn TextRun>> = first.iter().flatten().chain(second.iter().flatten()).collect();

    assert_eq!(all_runs.len(), 3);
    assert_eq!(all_runs.iter().filter(|run| same(run, &runs[1])).count(), 1);

    give_back(first, second);
}

/// Addition: splitting inside a shaped run gives two shaped runs that share
/// the glyph storage, and releases the original run.
#[test]
fn split_inside_shaped_run_splits_the_run() {
    let _scope = TextTestScope::new();

    let properties = run_properties(10.0);
    let source = SingleBufferTextSource::new("abcdef", properties.clone());
    let line =
        format_line(&source, 0, f64::INFINITY, &paragraph_properties(&properties, TextWrapping::NoWrap), None).unwrap();

    let runs: Vec<Rc<dyn TextRun>> = line.text_runs().to_vec();

    assert_eq!(runs.len(), 1);

    let (first, second, first_length) = split(&runs, 2);

    assert_eq!(first_length, 2);
    assert_content_preserved(&runs, &first, &second, first_length);

    let first_run = first.as_ref().unwrap()[0].downcast_ref::<ShapedTextRun>().unwrap();
    let second_run = second.as_ref().unwrap()[0].downcast_ref::<ShapedTextRun>().unwrap();

    assert_eq!(first_run.text().to_string_lossy(), "ab");
    assert_eq!(second_run.text().to_string_lossy(), "cdef");
    assert_eq!(first_run.shaped_buffer().length(), 2);
    assert_eq!(second_run.shaped_buffer().length(), 4);

    // The original run was released by the split.
    assert_eq!(runs[0].downcast_ref::<ShapedTextRun>().unwrap().shaped_buffer().length(), 0);

    give_back(first, second);
}
