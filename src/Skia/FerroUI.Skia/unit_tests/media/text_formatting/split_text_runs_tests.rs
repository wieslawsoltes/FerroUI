//! Port of upstream's `Media/TextFormatting/SplitTextRunsTests.cs` of the
//! Skia unit tests.
//!
//! Direct tests for `TextFormatterImpl::split_text_runs`. Calls the method
//! that is internal upstream (public in the port so these tests reach it) so
//! each branch can be exercised in isolation with synthetic `TextRun` stubs —
//! independent of the wrap algorithm that's its main caller. Many of these
//! scenarios are unreachable through the wrap path on its own (the wrap loop
//! carefully avoids requesting splits inside non-splittable runs), but the
//! method is also called by `TextCollapsingProperties` and the ellipsis
//! types, which have weaker invariants.
//!
//! Key invariants under test (must hold regardless of split position):
//!   * Sum of run lengths is preserved (no content lost or duplicated).
//!   * Concatenated text (across all runs in first ++ second) equals input.
//!   * Reported `first_length` equals the sum of lengths in first.
//!
//! Upstream's `SplitTextRuns(runs, length, pool, out firstLength)` is
//! `TextFormatterImpl::split_text_runs_with_length`, which returns the first
//! length next to the split result. Upstream returns the rented lists in a
//! `finally` block; here they are returned at the end of the test.

use ferroui_base::media::text_formatting::{FormattingObjectPool, RentedList, TextFormatterImpl, TextRun};
use std::any::Any;
use std::fmt;
use std::rc::Rc;

#[test]
fn split_inside_non_shaped_run_does_not_drop_run() {
    // Bug repro: a DrawableTextRun-like atomic run with length > 1, asked
    // to split at length=1 (strictly inside). Before the fix, the current
    // implementation dropped the run from both halves. After the fix it
    // must appear in either first or second.
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 1] = [TestStubRun::new("drawable", 3)];

    let (first, second, first_length) = split_text_runs(&runs, 1, pool);

    assert_content_preserved(&runs, &first, &second, first_length);

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_length_zero_returns_null_first_and_all_in_second() {
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 2] = [TestStubRun::new("a", 2), TestStubRun::new("b", 3)];

    let (first, second, first_length) = split_text_runs(&runs, 0, pool);

    assert!(first.is_none());
    assert!(second.is_some());
    assert_eq!(2, second.as_ref().unwrap().len());
    assert_eq!(0, first_length);
    assert_eq!(5, second.as_ref().unwrap().iter().map(|r| r.length()).sum::<i32>());

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_length_equals_total_puts_all_in_first() {
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 2] = [TestStubRun::new("a", 2), TestStubRun::new("b", 3)];

    let (first, second, first_length) = split_text_runs(&runs, 5, pool);

    assert!(first.is_some());
    assert_eq!(2, first.as_ref().unwrap().len());
    assert!(second.is_none());
    assert_eq!(5, first_length);
    assert_content_preserved(&runs, &first, &second, first_length);

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_length_past_total_puts_all_in_first() {
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 1] = [TestStubRun::new("a", 2)];

    let (first, second, first_length) = split_text_runs(&runs, 99, pool);

    assert!(first.is_some());
    assert_eq!(1, first.as_ref().unwrap().len());
    assert!(second.is_none());
    assert_eq!(2, first_length);

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_at_boundary_between_two_runs_goes_to_first_or_second_cleanly() {
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 2] = [TestStubRun::new("a", 2), TestStubRun::new("b", 3)];

    // length=2 means "everything up to and including the first run on first".
    let (first, second, first_length) = split_text_runs(&runs, 2, pool);

    assert!(first.is_some());
    assert_eq!(1, first.as_ref().unwrap().len());
    assert!(same(&runs[0], &first.as_ref().unwrap()[0]));
    assert!(second.is_some());
    assert_eq!(1, second.as_ref().unwrap().len());
    assert!(same(&runs[1], &second.as_ref().unwrap()[0]));
    assert_eq!(2, first_length);
    assert_content_preserved(&runs, &first, &second, first_length);

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_before_drawable_that_does_not_fit_puts_drawable_in_second() {
    // [text(2), drawable(1), text(2)] split at length=2.
    // Wrap normally chooses currentLength==length here ("drawable doesn't fit
    // on this line, push to next"). The == branch in SplitTextRuns already
    // handles this correctly today — assert that it stays correct.
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 3] =
        [TestStubRun::new("ab", 2), TestStubRun::new("X", 1), TestStubRun::new("cd", 2)];

    let (first, second, first_length) = split_text_runs(&runs, 2, pool);

    assert_eq!(2, first_length);
    assert_content_preserved(&runs, &first, &second, first_length);
    assert_eq!(1, first.as_ref().unwrap().len());
    assert_eq!(2, second.as_ref().unwrap().len());
    assert!(same(&runs[1], &second.as_ref().unwrap()[0])); // drawable at start of second

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_strictly_inside_non_shaped_run_snaps_before_it() {
    // [text(2), drawable(3), text(2)] split at length=3 — strictly inside
    // the drawable. The drawable is atomic, so the split must snap to a
    // boundary. The current contract: snap BEFORE the drawable, so
    // firstLength is shorter than requested but content is preserved.
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 3] =
        [TestStubRun::new("ab", 2), TestStubRun::new("XXX", 3), TestStubRun::new("cd", 2)];

    let (first, second, first_length) = split_text_runs(&runs, 3, pool);

    assert_content_preserved(&runs, &first, &second, first_length);
    assert!(
        first_length == 2 || first_length == 5,
        "Expected firstLength to snap to 2 (before drawable) or 5 (after drawable); got {first_length}."
    );

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_strictly_inside_non_shaped_run_at_start_of_list_overflows() {
    // [drawable(5)] split at length=2 — the drawable is the first run, has
    // no content before it, and is bigger than the requested length. If we
    // snapped before, first would be empty and the caller would loop
    // forever. The contract here is to overflow the drawable into first
    // (the same "include at least one cluster" rule the wrap loop has for
    // ShapedTextRuns at the start of a line).
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 1] = [TestStubRun::new("XXXXX", 5)];

    let (first, second, first_length) = split_text_runs(&runs, 2, pool);

    assert_content_preserved(&runs, &first, &second, first_length);
    assert_eq!(5, first_length); // overflow
    assert!(first.is_some());
    assert_eq!(1, first.as_ref().unwrap().len());
    assert!(same(&runs[0], &first.as_ref().unwrap()[0]));

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_at_boundary_before_drawable_mid_list() {
    // [shape(3), drawable(2), shape(3)] split at length=3 — boundary at
    // end of first shape. == branch.
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 3] =
        [TestStubRun::new("AAA", 3), TestStubRun::new("XX", 2), TestStubRun::new("BBB", 3)];

    let (first, second, first_length) = split_text_runs(&runs, 3, pool);

    assert_eq!(3, first_length);
    assert_content_preserved(&runs, &first, &second, first_length);

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_at_boundary_after_drawable_mid_list() {
    // Boundary at end of drawable. == branch.
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 3] =
        [TestStubRun::new("AAA", 3), TestStubRun::new("XX", 2), TestStubRun::new("BBB", 3)];

    let (first, second, first_length) = split_text_runs(&runs, 5, pool);

    assert_eq!(5, first_length);
    assert_content_preserved(&runs, &first, &second, first_length);
    assert_eq!(2, first.as_ref().unwrap().len());
    assert_eq!(1, second.as_ref().unwrap().len());

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

#[test]
fn split_with_zero_length_run_inside_does_not_drop_it() {
    // Zero-length runs (e.g. TextEndOfParagraph variants) appear after
    // shaped runs. Splitting at the boundary should keep the zero-length
    // run somewhere — not silently discard it.
    let pool = FormattingObjectPool::instance();
    let runs: [Rc<dyn TextRun>; 3] =
        [TestStubRun::new("ab", 2), TestStubRun::new("zero", 0), TestStubRun::new("cd", 2)];

    let (first, second, first_length) = split_text_runs(&runs, 2, pool);

    assert_eq!(2, first_length);
    // The zero-length run still has identity — assert it's in exactly one half.
    let all_runs: Vec<&Rc<dyn TextRun>> = first.iter().flatten().chain(second.iter().flatten()).collect();
    assert_eq!(3, all_runs.len());
    assert!(all_runs.iter().any(|run| same(run, &runs[1])));

    pool.text_run_lists.return_optional(first);
    pool.text_run_lists.return_optional(second);
}

type Runs = Option<RentedList<Rc<dyn TextRun>>>;

/// `var (first, second) = TextFormatterImpl.SplitTextRuns(runs, length, pool, out var firstLength)`.
fn split_text_runs(runs: &[Rc<dyn TextRun>], length: i32, pool: &FormattingObjectPool) -> (Runs, Runs, i32) {
    let (split, first_length) = TextFormatterImpl::split_text_runs_with_length(runs, length, pool);

    let (first, second) = split.deconstruct();

    (first, second, first_length)
}

/// `Assert.Same`.
fn same(x: &Rc<dyn TextRun>, y: &Rc<dyn TextRun>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(x), Rc::as_ptr(y))
}

/// Asserts the central invariant: sum of (first|second) run lengths
/// equals the input total, and `first_length` matches the
/// sum of first's run lengths. Any test where this fails means content
/// was lost or duplicated.
fn assert_content_preserved(input: &[Rc<dyn TextRun>], first: &Runs, second: &Runs, first_length: i32) {
    let input_total: i32 = input.iter().map(|r| r.length()).sum();
    let first_total: i32 = first.as_ref().map_or(0, |first| first.iter().map(|r| r.length()).sum());
    let second_total: i32 = second.as_ref().map_or(0, |second| second.iter().map(|r| r.length()).sum());

    assert_eq!(input_total, first_total + second_total);
    assert_eq!(first_total, first_length);
}

/// Minimal concrete `TextRun` for tests — neither a `ShapedTextRun` nor a
/// `DrawableTextRun` from the consumer perspective. Behaves like an atomic,
/// non-splittable run with a configurable length, which is exactly the class
/// of input that triggers the `SplitTextRuns` drop-current-run bug.
struct TestStubRun {
    name: &'static str,
    length: i32,
}

impl TestStubRun {
    fn new(name: &'static str, length: i32) -> Rc<dyn TextRun> {
        Rc::new(TestStubRun { name, length })
    }
}

impl TextRun for TestStubRun {
    fn length(&self) -> i32 {
        self.length
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl fmt::Display for TestStubRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TestStubRun({}, len={})", self.name, self.length)
    }
}
