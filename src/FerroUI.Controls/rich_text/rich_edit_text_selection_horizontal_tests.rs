//! Literal directed selections from the fresh-window system MSFTEDIT probe.
//! Sources: PR #243 runs 39618880985/39765696678, Windows jobs
//! 119286684228/119780463926, MSFTEDIT 6.2.26100.33648. Native flags/type and
//! synthetic terminal EOP remain separate contracts; these rows compare
//! accepted offsets and signed movement counts.
use super::*;
use crate::RichEditBox;
use ferroui_base::threading::Dispatcher;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug)]
struct HorizontalCase {
    right: bool,
    unit: TextRangeUnit,
    count: i32,
    extend: bool,
    before: (usize, usize),
    after: (usize, usize),
    moved: i32,
}

const fn case(
    right: bool,
    unit: TextRangeUnit,
    count: i32,
    extend: bool,
    before: (usize, usize),
    after: (usize, usize),
    moved: i32,
) -> HorizontalCase {
    HorizontalCase {
        right,
        unit,
        count,
        extend,
        before,
        after,
        moved,
    }
}

fn assert_horizontal_cases(text: &str, cases: &[HorizontalCase]) {
    for case in cases {
        // Match the native isolation policy: every horizontal row starts from
        // a fresh control, so prior affinity cannot contaminate it.
        let document = RichEditTextDocument::new(RichTextDocument::from_text(text));
        let selection = document.selection();
        selection.set_range(case.before.0, case.before.1).unwrap();
        let moved = if case.right {
            selection.move_right(case.unit, case.count, case.extend)
        } else {
            selection.move_left(case.unit, case.count, case.extend)
        };
        assert_eq!(moved.unwrap(), case.moved, "{text:?}: {case:?}");
        assert_eq!(
            selection.selection(),
            EditorSelection::new(case.after.0, case.after.1),
            "{text:?}: {case:?}"
        );
        assert_eq!(document.text(), text);
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_selection_horizontal_matches_216_system_msftedit_observations() {
    let _scope = Dispatcher::unit_test_scope();
    use TextRangeUnit::{Character as C, Word as W};
    // The other four of the 220 recorded rows select native's synthetic final
    // EOP. Their intentional model boundary is tested separately below.
    let fixtures: &[(&str, &[HorizontalCase])] = &[
        (
            "one two three",
            &[
                case(false, C, 0, false, (1, 6), (1, 6), 0),
                case(false, C, 1, false, (1, 6), (1, 1), 1),
                case(false, C, 2, false, (1, 6), (0, 0), 2),
                case(false, C, -1, false, (1, 6), (6, 6), -1),
                case(true, C, 0, false, (1, 6), (1, 6), 0),
                case(true, C, 1, false, (1, 6), (6, 6), 1),
                case(true, C, 2, false, (1, 6), (7, 7), 2),
                case(true, C, -1, false, (1, 6), (1, 1), -1),
                case(false, C, 0, true, (1, 6), (1, 6), 0),
                case(false, C, 1, true, (1, 6), (1, 5), 1),
                case(false, C, 2, true, (1, 6), (1, 4), 2),
                case(false, C, -1, true, (1, 6), (1, 7), -1),
                case(true, C, 0, true, (1, 6), (1, 6), 0),
                case(true, C, 1, true, (1, 6), (1, 7), 1),
                case(true, C, 2, true, (1, 6), (1, 8), 2),
                case(true, C, -1, true, (1, 6), (1, 5), -1),
                case(false, W, 0, false, (1, 6), (1, 6), 0),
                case(false, W, 1, false, (1, 6), (0, 0), 1),
                case(false, W, 2, false, (1, 6), (0, 0), 1),
                case(false, W, -1, false, (1, 6), (8, 8), -1),
                case(true, W, 0, false, (1, 6), (1, 6), 0),
                case(true, W, 1, false, (1, 6), (8, 8), 1),
                case(true, W, 2, false, (1, 6), (13, 13), 2),
                case(true, W, -1, false, (1, 6), (0, 0), -1),
                case(false, W, 0, true, (1, 6), (1, 6), 0),
                case(false, W, 1, true, (1, 6), (1, 4), 1),
                case(false, W, 2, true, (1, 6), (1, 0), 2),
                case(false, W, -1, true, (1, 6), (1, 8), -1),
                case(true, W, 0, true, (1, 6), (1, 6), 0),
                case(true, W, 1, true, (1, 6), (1, 8), 1),
                case(true, W, 2, true, (1, 6), (1, 13), 2),
                case(true, W, -1, true, (1, 6), (1, 4), -1),
                case(false, C, 0, false, (6, 1), (6, 1), 0),
                case(false, C, 1, false, (6, 1), (1, 1), 1),
                case(false, C, 2, false, (6, 1), (0, 0), 2),
                case(false, C, -1, false, (6, 1), (6, 6), -1),
                case(true, C, 0, false, (6, 1), (6, 1), 0),
                case(true, C, 1, false, (6, 1), (6, 6), 1),
                case(true, C, 2, false, (6, 1), (7, 7), 2),
                case(true, C, -1, false, (6, 1), (1, 1), -1),
                case(false, C, 0, true, (6, 1), (6, 1), 0),
                case(false, C, 1, true, (6, 1), (6, 0), 1),
                case(false, C, 2, true, (6, 1), (6, 0), 1),
                case(false, C, -1, true, (6, 1), (6, 2), -1),
                case(true, C, 0, true, (6, 1), (6, 1), 0),
                case(true, C, 1, true, (6, 1), (6, 2), 1),
                case(true, C, 2, true, (6, 1), (6, 3), 2),
                case(true, C, -1, true, (6, 1), (6, 0), -1),
                case(false, W, 0, false, (6, 1), (6, 1), 0),
                case(false, W, 1, false, (6, 1), (0, 0), 1),
                case(false, W, 2, false, (6, 1), (0, 0), 1),
                case(false, W, -1, false, (6, 1), (8, 8), -1),
                case(true, W, 0, false, (6, 1), (6, 1), 0),
                case(true, W, 1, false, (6, 1), (8, 8), 1),
                case(true, W, 2, false, (6, 1), (13, 13), 2),
                case(true, W, -1, false, (6, 1), (0, 0), -1),
                case(false, W, 0, true, (6, 1), (6, 1), 0),
                case(false, W, 1, true, (6, 1), (6, 0), 1),
                case(false, W, 2, true, (6, 1), (6, 0), 1),
                case(false, W, -1, true, (6, 1), (6, 4), -1),
                case(true, W, 0, true, (6, 1), (6, 1), 0),
                case(true, W, 1, true, (6, 1), (6, 4), 1),
                case(true, W, 2, true, (6, 1), (6, 8), 2),
                case(true, W, -1, true, (6, 1), (6, 0), -1),
                case(false, C, 1, false, (5, 5), (4, 4), 1),
                case(false, C, -1, false, (5, 5), (6, 6), -1),
                case(true, C, 1, false, (5, 5), (6, 6), 1),
                case(true, C, -1, false, (5, 5), (4, 4), -1),
                case(false, C, 1, true, (5, 5), (5, 4), 1),
                case(false, C, -1, true, (5, 5), (5, 6), -1),
                case(true, C, 1, true, (5, 5), (5, 6), 1),
                case(true, C, -1, true, (5, 5), (5, 4), -1),
                case(false, W, 1, false, (5, 5), (4, 4), 1),
                case(false, W, -1, false, (5, 5), (8, 8), -1),
                case(true, W, 1, false, (5, 5), (8, 8), 1),
                case(true, W, -1, false, (5, 5), (4, 4), -1),
                case(false, W, 1, true, (5, 5), (5, 4), 1),
                case(false, W, -1, true, (5, 5), (5, 8), -1),
                case(true, W, 1, true, (5, 5), (5, 8), 1),
                case(true, W, -1, true, (5, 5), (5, 4), -1),
                case(false, C, 1, false, (0, 0), (0, 0), 0),
                case(false, C, 1, true, (0, 0), (0, 0), 0),
                case(false, W, 1, false, (0, 0), (0, 0), 0),
                case(false, W, 1, true, (0, 0), (0, 0), 0),
                case(true, C, 1, false, (13, 13), (13, 13), 0),
                case(true, W, 1, false, (13, 13), (13, 13), 0),
            ],
        ),
        (
            "A😀é👩‍👩‍👦B",
            &[
                case(false, C, 1, false, (1, 1), (0, 0), 1),
                case(true, C, 1, false, (1, 1), (3, 3), 1),
                case(false, C, 1, true, (1, 1), (1, 0), 1),
                case(true, C, 1, true, (1, 1), (1, 3), 1),
                case(false, C, 1, false, (3, 3), (1, 1), 1),
                case(true, C, 1, false, (3, 3), (5, 5), 1),
                case(false, C, 1, true, (3, 3), (3, 1), 1),
                case(true, C, 1, true, (3, 3), (3, 5), 1),
                case(false, C, 1, false, (4, 4), (3, 3), 1),
                case(true, C, 1, false, (4, 4), (5, 5), 1),
                case(false, C, 1, true, (4, 4), (4, 3), 1),
                case(true, C, 1, true, (4, 4), (4, 5), 1),
                case(false, C, 1, false, (5, 5), (3, 3), 1),
                case(true, C, 1, false, (5, 5), (13, 13), 1),
                case(false, C, 1, true, (5, 5), (5, 3), 1),
                case(true, C, 1, true, (5, 5), (5, 13), 1),
                case(false, C, 1, false, (13, 13), (5, 5), 1),
                case(true, C, 1, false, (13, 13), (14, 14), 1),
                case(false, C, 1, true, (13, 13), (13, 5), 1),
                case(true, C, 1, true, (13, 13), (13, 14), 1),
            ],
        ),
        (
            "one, two\tthree\n四五 six",
            &[
                case(false, W, 1, false, (3, 3), (0, 0), 1),
                case(true, W, 1, false, (3, 3), (5, 5), 1),
                case(false, W, 1, true, (3, 3), (3, 0), 1),
                case(true, W, 1, true, (3, 3), (3, 5), 1),
                case(false, W, 1, false, (4, 4), (3, 3), 1),
                case(true, W, 1, false, (4, 4), (5, 5), 1),
                case(false, W, 1, true, (4, 4), (4, 3), 1),
                case(true, W, 1, true, (4, 4), (4, 5), 1),
                case(false, W, 1, false, (8, 8), (5, 5), 1),
                case(true, W, 1, false, (8, 8), (9, 9), 1),
                case(false, W, 1, true, (8, 8), (8, 5), 1),
                case(true, W, 1, true, (8, 8), (8, 9), 1),
                case(false, W, 1, false, (14, 14), (9, 9), 1),
                case(true, W, 1, false, (14, 14), (15, 15), 1),
                case(false, W, 1, true, (14, 14), (14, 9), 1),
                case(true, W, 1, true, (14, 14), (14, 15), 1),
                case(false, W, 1, false, (15, 15), (14, 14), 1),
                case(true, W, 1, false, (15, 15), (16, 16), 1),
                case(false, W, 1, true, (15, 15), (15, 14), 1),
                case(true, W, 1, true, (15, 15), (15, 16), 1),
                case(false, W, 1, false, (17, 17), (16, 16), 1),
                case(true, W, 1, false, (17, 17), (18, 18), 1),
                case(false, W, 1, true, (17, 17), (17, 16), 1),
                case(true, W, 1, true, (17, 17), (17, 18), 1),
            ],
        ),
        (
            "abc אבג xyz",
            &[
                case(false, C, 1, false, (4, 4), (3, 3), 1),
                case(true, C, 1, false, (4, 4), (5, 5), 1),
                case(false, C, 1, true, (4, 4), (4, 3), 1),
                case(true, C, 1, true, (4, 4), (4, 5), 1),
                case(false, W, 1, false, (4, 4), (0, 0), 1),
                case(true, W, 1, false, (4, 4), (8, 8), 1),
                case(false, W, 1, true, (4, 4), (4, 0), 1),
                case(true, W, 1, true, (4, 4), (4, 8), 1),
                case(false, C, 1, false, (5, 5), (4, 4), 1),
                case(true, C, 1, false, (5, 5), (6, 6), 1),
                case(false, C, 1, true, (5, 5), (5, 4), 1),
                case(true, C, 1, true, (5, 5), (5, 6), 1),
                case(false, W, 1, false, (5, 5), (4, 4), 1),
                case(true, W, 1, false, (5, 5), (8, 8), 1),
                case(false, W, 1, true, (5, 5), (5, 4), 1),
                case(true, W, 1, true, (5, 5), (5, 8), 1),
                case(false, C, 1, false, (7, 7), (6, 6), 1),
                case(true, C, 1, false, (7, 7), (8, 8), 1),
                case(false, C, 1, true, (7, 7), (7, 6), 1),
                case(true, C, 1, true, (7, 7), (7, 8), 1),
                case(false, W, 1, false, (7, 7), (4, 4), 1),
                case(true, W, 1, false, (7, 7), (8, 8), 1),
                case(false, W, 1, true, (7, 7), (7, 4), 1),
                case(true, W, 1, true, (7, 7), (7, 8), 1),
                case(false, C, 1, false, (2, 6), (2, 2), 1),
                case(true, C, 1, false, (2, 6), (6, 6), 1),
                case(false, C, 1, true, (2, 6), (2, 5), 1),
                case(true, C, 1, true, (2, 6), (2, 7), 1),
                case(false, W, 1, false, (2, 6), (0, 0), 1),
                case(true, W, 1, false, (2, 6), (8, 8), 1),
                case(false, W, 1, true, (2, 6), (2, 4), 1),
                case(true, W, 1, true, (2, 6), (2, 8), 1),
                case(false, C, 1, false, (6, 2), (2, 2), 1),
                case(true, C, 1, false, (6, 2), (6, 6), 1),
                case(false, C, 1, true, (6, 2), (6, 1), 1),
                case(true, C, 1, true, (6, 2), (6, 3), 1),
                case(false, W, 1, false, (6, 2), (0, 0), 1),
                case(true, W, 1, false, (6, 2), (8, 8), 1),
                case(false, W, 1, true, (6, 2), (6, 0), 1),
                case(true, W, 1, true, (6, 2), (6, 4), 1),
            ],
        ),
        (
            "abc אבג xyz",
            &[
                case(false, C, 1, false, (4, 4), (3, 3), 1),
                case(true, C, 1, false, (4, 4), (5, 5), 1),
                case(false, C, 1, true, (4, 4), (4, 3), 1),
                case(true, C, 1, true, (4, 4), (4, 5), 1),
                case(false, W, 1, false, (4, 4), (0, 0), 1),
                case(true, W, 1, false, (4, 4), (8, 8), 1),
                case(false, W, 1, true, (4, 4), (4, 0), 1),
                case(true, W, 1, true, (4, 4), (4, 8), 1),
                case(false, C, 1, false, (5, 5), (4, 4), 1),
                case(true, C, 1, false, (5, 5), (6, 6), 1),
                case(false, C, 1, true, (5, 5), (5, 4), 1),
                case(true, C, 1, true, (5, 5), (5, 6), 1),
                case(false, W, 1, false, (5, 5), (4, 4), 1),
                case(true, W, 1, false, (5, 5), (8, 8), 1),
                case(false, W, 1, true, (5, 5), (5, 4), 1),
                case(true, W, 1, true, (5, 5), (5, 8), 1),
                case(false, C, 1, false, (7, 7), (6, 6), 1),
                case(true, C, 1, false, (7, 7), (8, 8), 1),
                case(false, C, 1, true, (7, 7), (7, 6), 1),
                case(true, C, 1, true, (7, 7), (7, 8), 1),
                case(false, W, 1, false, (7, 7), (4, 4), 1),
                case(true, W, 1, false, (7, 7), (8, 8), 1),
                case(false, W, 1, true, (7, 7), (7, 4), 1),
                case(true, W, 1, true, (7, 7), (7, 8), 1),
                case(false, C, 1, false, (2, 6), (2, 2), 1),
                case(true, C, 1, false, (2, 6), (6, 6), 1),
                case(false, C, 1, true, (2, 6), (2, 5), 1),
                case(true, C, 1, true, (2, 6), (2, 7), 1),
                case(false, W, 1, false, (2, 6), (0, 0), 1),
                case(true, W, 1, false, (2, 6), (8, 8), 1),
                case(false, W, 1, true, (2, 6), (2, 4), 1),
                case(true, W, 1, true, (2, 6), (2, 8), 1),
                case(false, C, 1, false, (6, 2), (2, 2), 1),
                case(true, C, 1, false, (6, 2), (6, 6), 1),
                case(false, C, 1, true, (6, 2), (6, 1), 1),
                case(true, C, 1, true, (6, 2), (6, 3), 1),
                case(false, W, 1, false, (6, 2), (0, 0), 1),
                case(true, W, 1, false, (6, 2), (8, 8), 1),
                case(false, W, 1, true, (6, 2), (6, 0), 1),
                case(true, W, 1, true, (6, 2), (6, 4), 1),
            ],
        ),
        (
            "",
            &[
                case(false, C, 1, false, (0, 0), (0, 0), 0),
                case(true, C, 1, false, (0, 0), (0, 0), 0),
                case(false, C, 1, true, (0, 0), (0, 0), 0),
                case(false, W, 1, false, (0, 0), (0, 0), 0),
                case(true, W, 1, false, (0, 0), (0, 0), 0),
                case(false, W, 1, true, (0, 0), (0, 0), 0),
            ],
        ),
    ];
    let mut checked = 0;
    for (text, cases) in fixtures {
        assert_horizontal_cases(text, cases);
        checked += cases.len();
    }
    assert_eq!(checked, 216);
}

#[test]
fn rich_selection_horizontal_matches_78_native_boundary_observations() {
    let _scope = Dispatcher::unit_test_scope();
    use TextRangeUnit::{Character as C, Word as W};
    // Additional fresh-window rows from run 39765696678, job 119780463926:
    // aligned/reversed Word endpoints consume the collapse unit; Character
    // collapse preserves a scalar endpoint inside a combining sequence. Two
    // extra terminal-EOP rows are covered by the explicit model-boundary test.
    let fixtures: &[(&str, &[HorizontalCase])] = &[
        (
            "one two three",
            &[
                case(false, W, -2, false, (0, 4), (8, 8), -2),
                case(false, W, -1, false, (0, 4), (4, 4), -1),
                case(false, W, 1, false, (0, 4), (0, 0), 1),
                case(false, W, 2, false, (0, 4), (0, 0), 1),
                case(true, W, -2, false, (0, 4), (0, 0), -1),
                case(true, W, -1, false, (0, 4), (0, 0), -1),
                case(true, W, 1, false, (0, 4), (4, 4), 1),
                case(true, W, 2, false, (0, 4), (8, 8), 2),
                case(false, W, -2, true, (0, 4), (0, 13), -2),
                case(false, W, -1, true, (0, 4), (0, 8), -1),
                case(false, W, 1, true, (0, 4), (0, 0), 1),
                case(false, W, 2, true, (0, 4), (0, 0), 1),
                case(true, W, -2, true, (0, 4), (0, 0), -1),
                case(true, W, -1, true, (0, 4), (0, 0), -1),
                case(true, W, 1, true, (0, 4), (0, 8), 1),
                case(true, W, 2, true, (0, 4), (0, 13), 2),
                case(false, W, -2, false, (4, 8), (13, 13), -2),
                case(false, W, -1, false, (4, 8), (8, 8), -1),
                case(false, W, 1, false, (4, 8), (4, 4), 1),
                case(false, W, 2, false, (4, 8), (0, 0), 2),
                case(true, W, -2, false, (4, 8), (0, 0), -2),
                case(true, W, -1, false, (4, 8), (4, 4), -1),
                case(true, W, 1, false, (4, 8), (8, 8), 1),
                case(true, W, 2, false, (4, 8), (13, 13), 2),
                case(false, W, -1, true, (4, 8), (4, 13), -1),
                case(false, W, 1, true, (4, 8), (4, 4), 1),
                case(false, W, 2, true, (4, 8), (4, 0), 2),
                case(true, W, -2, true, (4, 8), (4, 0), -2),
                case(true, W, -1, true, (4, 8), (4, 4), -1),
                case(true, W, 1, true, (4, 8), (4, 13), 1),
                case(false, W, -2, false, (4, 0), (8, 8), -2),
                case(false, W, -1, false, (4, 0), (4, 4), -1),
                case(false, W, 1, false, (4, 0), (0, 0), 1),
                case(false, W, 2, false, (4, 0), (0, 0), 1),
                case(true, W, -2, false, (4, 0), (0, 0), -1),
                case(true, W, -1, false, (4, 0), (0, 0), -1),
                case(true, W, 1, false, (4, 0), (4, 4), 1),
                case(true, W, 2, false, (4, 0), (8, 8), 2),
                case(false, W, -2, true, (4, 0), (4, 8), -2),
                case(false, W, -1, true, (4, 0), (4, 4), -1),
                case(false, W, 1, true, (4, 0), (4, 0), 0),
                case(false, W, 2, true, (4, 0), (4, 0), 0),
                case(true, W, -2, true, (4, 0), (4, 0), 0),
                case(true, W, -1, true, (4, 0), (4, 0), 0),
                case(true, W, 1, true, (4, 0), (4, 4), 1),
                case(true, W, 2, true, (4, 0), (4, 8), 2),
                case(false, W, -2, false, (8, 4), (13, 13), -2),
                case(false, W, -1, false, (8, 4), (8, 8), -1),
                case(false, W, 1, false, (8, 4), (4, 4), 1),
                case(false, W, 2, false, (8, 4), (0, 0), 2),
                case(true, W, -2, false, (8, 4), (0, 0), -2),
                case(true, W, -1, false, (8, 4), (4, 4), -1),
                case(true, W, 1, false, (8, 4), (8, 8), 1),
                case(true, W, 2, false, (8, 4), (13, 13), 2),
                case(false, W, -2, true, (8, 4), (8, 13), -2),
                case(false, W, -1, true, (8, 4), (8, 8), -1),
                case(false, W, 1, true, (8, 4), (8, 0), 1),
                case(false, W, 2, true, (8, 4), (8, 0), 1),
                case(true, W, -2, true, (8, 4), (8, 0), -1),
                case(true, W, -1, true, (8, 4), (8, 0), -1),
                case(true, W, 1, true, (8, 4), (8, 8), 1),
                case(true, W, 2, true, (8, 4), (8, 13), 2),
            ],
        ),
        (
            "A😀é👩‍👩‍👦B",
            &[
                case(false, C, 1, false, (1, 4), (1, 1), 1),
                case(false, C, 2, false, (1, 4), (0, 0), 2),
                case(true, C, 1, false, (1, 4), (4, 4), 1),
                case(true, C, 2, false, (1, 4), (5, 5), 2),
                case(false, C, 1, true, (1, 4), (1, 3), 1),
                case(false, C, 2, true, (1, 4), (1, 1), 2),
                case(true, C, 1, true, (1, 4), (1, 5), 1),
                case(true, C, 2, true, (1, 4), (1, 13), 2),
                case(false, C, 1, false, (4, 1), (1, 1), 1),
                case(false, C, 2, false, (4, 1), (0, 0), 2),
                case(true, C, 1, false, (4, 1), (4, 4), 1),
                case(true, C, 2, false, (4, 1), (5, 5), 2),
                case(false, C, 1, true, (4, 1), (4, 0), 1),
                case(false, C, 2, true, (4, 1), (4, 0), 1),
                case(true, C, 1, true, (4, 1), (4, 3), 1),
                case(true, C, 2, true, (4, 1), (4, 5), 2),
            ],
        ),
    ];
    let mut checked = 0;
    for (text, cases) in fixtures {
        assert_horizontal_cases(text, cases);
        checked += cases.len();
    }
    assert_eq!(checked, 78);
}

#[test]
fn rich_selection_horizontal_has_no_synthetic_terminal_eop() {
    let _scope = Dispatcher::unit_test_scope();
    for text in ["", "one two three"] {
        let document = RichEditTextDocument::new(RichTextDocument::from_text(text));
        let end = document.text_length();
        for unit in [TextRangeUnit::Character, TextRangeUnit::Word] {
            document.selection().set_range(end, end).unwrap();
            // The corresponding native row returns one and selects end+1.
            // Rust has no such character and must not invent a range or change.
            assert_eq!(document.selection().move_right(unit, 1, true).unwrap(), 0);
            assert_eq!(
                document.selection().selection(),
                EditorSelection::caret(end)
            );
            assert_eq!(document.text(), text);
        }
    }
    // The expanded native probe adds two more EOP observations: extending
    // [4,8] right by two words reaches 14 with delta 2 (Left(-2) reports -2).
    // The model instead saturates at the real text end after one accepted unit.
    let document = RichEditTextDocument::new(RichTextDocument::from_text("one two three"));
    for right in [false, true] {
        document.selection().set_range(4, 8).unwrap();
        let moved = if right {
            document
                .selection()
                .move_right(TextRangeUnit::Word, 2, true)
        } else {
            document
                .selection()
                .move_left(TextRangeUnit::Word, -2, true)
        };
        assert_eq!(moved.unwrap(), if right { 1 } else { -1 });
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(4, 13)
        );
        assert_eq!(document.text(), "one two three");
    }
}

#[test]
fn rich_selection_horizontal_character_keeps_scalar_range_contract_independent() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::from_text("Ae\u{301}B"));
    document.selection().set_range(1, 1).unwrap();
    assert_eq!(
        document
            .selection()
            .move_by(TextRangeUnit::Character, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(2));
    document.selection().set_range(1, 1).unwrap();
    assert_eq!(
        document
            .selection()
            .move_right(TextRangeUnit::Character, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(3));
}

#[test]
fn rich_selection_horizontal_validates_units_before_zero_and_preserves_history() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::from_text("one two"));
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    let snapshot = document.document().snapshot();
    let before = document.selection().selection();
    for unit in [
        TextRangeUnit::Line,
        TextRangeUnit::Paragraph,
        TextRangeUnit::Sentence,
        TextRangeUnit::Story,
        TextRangeUnit::HardParagraph,
        TextRangeUnit::Cluster,
    ] {
        for count in [0, 1, i32::MIN, i32::MAX] {
            assert!(document.selection().move_left(unit, count, true).is_err());
            assert!(document.selection().move_right(unit, count, false).is_err());
        }
    }
    assert_eq!(document.selection().selection(), before);
    assert_eq!(document.document().snapshot(), snapshot);
    assert!(document.can_redo());
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_horizontal_extreme_counts_saturate_to_real_boundaries() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::from_text("A😀e\u{301}B"));
    for extend in [false, true] {
        document.selection().set_range(0, 0).unwrap();
        assert_eq!(
            document
                .selection()
                .move_left(TextRangeUnit::Character, i32::MIN, extend)
                .unwrap(),
            -4
        );
        assert_eq!(document.selection().selection().active, 6);
        document.selection().set_range(6, 6).unwrap();
        assert_eq!(
            document
                .selection()
                .move_right(TextRangeUnit::Character, i32::MIN, extend)
                .unwrap(),
            -4
        );
        assert_eq!(document.selection().selection().active, 0);
        document.selection().set_range(0, 0).unwrap();
        assert_eq!(
            document
                .selection()
                .move_right(TextRangeUnit::Character, i32::MAX, extend)
                .unwrap(),
            4
        );
        assert_eq!(document.selection().selection().active, 6);
    }
}

#[test]
fn rich_selection_horizontal_cancellation_preserves_selection_document_and_redo() {
    let _scope = Dispatcher::unit_test_scope();
    let owner = RichEditBox::new();
    owner.set_text("A😀e\u{301}B").unwrap();
    let document = owner.text_document();
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    owner.select(1, 1);
    owner.selection_changing(|_, args| args.set_cancel(true));
    let snapshot = document.document().snapshot();
    let revision = document.document().revision();
    assert_eq!(
        document
            .selection()
            .move_right(TextRangeUnit::Character, 2, true)
            .unwrap(),
        0
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(1));
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(document.document().revision(), revision);
    assert!(document.can_redo());
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_horizontal_redirection_counts_graphemes_and_command_direction() {
    let _scope = Dispatcher::unit_test_scope();
    for right in [false, true] {
        let owner = RichEditBox::new();
        owner
            .set_text("A😀e\u{301}👩\u{200d}👩\u{200d}👦B")
            .unwrap();
        owner.select(3, 3);
        let document = owner.text_document();
        let weak = owner.downgrade();
        owner.selection_changing(move |_, args| {
            weak.upgrade().unwrap().select(3, 13);
            args.set_cancel(true);
        });
        let moved = if right {
            document
                .selection()
                .move_right(TextRangeUnit::Character, 1, true)
        } else {
            document
                .selection()
                .move_left(TextRangeUnit::Character, 1, true)
        };
        assert_eq!(moved.unwrap(), if right { 2 } else { -2 });
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(3, 13)
        );
    }
}

#[test]
fn rich_selection_horizontal_callback_model_edit_rejects_stale_count() {
    let _scope = Dispatcher::unit_test_scope();
    let owner = RichEditBox::new();
    owner.set_text("one two three").unwrap();
    owner.select(1, 1);
    let document = owner.text_document();
    let model = document.document();
    let batch = model.begin_edit();
    let callback_model = model.clone();
    let changed = Rc::new(Cell::new(false));
    owner.selection_changing(move |_, _| {
        if !changed.replace(true) {
            callback_model
                .replace_text(0..callback_model.text_length(), "external")
                .unwrap();
        }
    });
    assert!(matches!(
        document
            .selection()
            .move_right(TextRangeUnit::Word, 1, true),
        Err(EditError::ReentrantEdit)
    ));
    assert_eq!(document.text(), "external");
    assert!(!document.can_undo());
    assert!(!document.can_redo());
    drop(batch);
}

#[test]
fn rich_selection_horizontal_nested_boundary_command_owns_vertical_affinity() {
    let _scope = crate::test_support::test_scope();
    let _text = ferroui_base::media::text_formatting::testing::TextTestScope::new();
    let owner = RichEditBox::new();
    owner.set_text("aaaaaa\na").unwrap();
    let document = owner.text_document();
    let view = owner.text_presenter().upcast::<RichTextBlock>();
    document.editor().set_text_view(Some(view.clone())).unwrap();
    view.measure(ferroui_base::Size::new(300.0, 500.0));
    view.arrange(ferroui_base::Rect::new(0.0, 0.0, 300.0, 500.0));
    owner.select(5, 5);
    let nested = document.selection();
    let changed = Rc::new(Cell::new(false));
    let observed = changed.clone();
    owner.selection_changing(move |_, _| {
        if !changed.replace(true) {
            assert_eq!(nested.selection(), EditorSelection::caret(8));
            // A nonzero horizontal command at the story boundary owns the
            // reached caret, even when its source offsets do not change.
            assert_eq!(
                nested
                    .move_right(TextRangeUnit::Character, 1, false)
                    .unwrap(),
                0
            );
        }
    });
    assert_eq!(
        document
            .selection()
            .move_down(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert!(observed.get());
    assert_eq!(
        document
            .selection()
            .move_up(TextRangeUnit::Line, 1, false)
            .unwrap(),
        1
    );
    assert_eq!(document.selection().selection(), EditorSelection::caret(1));
}
