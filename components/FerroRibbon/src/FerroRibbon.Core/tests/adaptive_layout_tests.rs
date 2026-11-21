//! Port of tests/RibbonSpace.Core.Tests/AdaptiveLayoutTests.cs and its two layout regressions.
use RibbonGroupState as S;
use RibbonSimplifiedVisibility as V;
use ferroribbon_core::{layout::*, *};
fn g(l: f64, m: f64, s: f64, c: f64) -> RibbonGroupLayoutInfo {
    RibbonGroupLayoutInfo::new(vec![l, m, s, c])
}
fn normal() -> RibbonGroupLayoutInfo {
    g(100., 80., 60., 40.)
}
#[test]
fn everything_stays_large_when_it_fits() {
    let r = RibbonAdaptiveLayout::compute(&[normal(), normal()], 500.);
    assert!(r.fits);
    assert_eq!(r.states, vec![S::Large; 2]);
    assert_eq!(r.total_width, 200.);
}
#[test]
fn rightmost_group_reduces_first() {
    let r = RibbonAdaptiveLayout::compute(&[normal(), normal()], 185.);
    assert_eq!(r.states, [S::Large, S::Medium]);
    assert!(r.fits);
}
#[test]
fn all_groups_reach_medium_before_any_reaches_small() {
    let r = RibbonAdaptiveLayout::compute(&[normal(), normal()], 145.);
    assert_eq!(r.states, [S::Medium, S::Small]);
}
#[test]
fn reduction_order_overrides_position() {
    let mut first = normal();
    first.reduction_order = 5;
    let r = RibbonAdaptiveLayout::compute(&[first, normal()], 185.);
    assert_eq!(r.states, [S::Medium, S::Large]);
}
#[test]
fn groups_collapse_last_and_respect_can_collapse() {
    let mut last = normal();
    last.can_collapse = false;
    let r = RibbonAdaptiveLayout::compute(&[normal(), last], 100.);
    assert_eq!(r.states, [S::Collapsed, S::Small]);
    assert!(r.fits);
}
#[test]
fn reports_when_nothing_fits() {
    let r = RibbonAdaptiveLayout::compute(&[normal(), normal()], 10.);
    assert!(!r.fits);
    assert_eq!(r.states, vec![S::Collapsed; 2]);
    assert_eq!(r.total_width, 80.);
}
#[test]
fn skips_steps_that_do_not_reduce_width() {
    let r =
        RibbonAdaptiveLayout::compute(&[g(100., 70., 60., 40.), g(100., 100., 100., 40.)], 175.);
    assert_eq!(r.states[0], S::Medium);
    assert_ne!(r.states[1], S::Collapsed);
    assert!(r.fits);
}
#[test]
fn spacing_is_included() {
    let r = RibbonAdaptiveLayout::compute_with_spacing(&[normal(), normal()], 205., 10.);
    assert_eq!(r.states[1], S::Medium);
    assert_eq!(r.total_width, 190.);
}
#[test]
fn large_items_take_full_columns_and_small_items_stack_in_threes() {
    let items = [
        RibbonArrangeItem::new(50., 66., true),
        RibbonArrangeItem::new(60., 22., false),
        RibbonArrangeItem::new(70., 22., false),
        RibbonArrangeItem::new(40., 22., false),
        RibbonArrangeItem::new(30., 22., false),
    ];
    let r = RibbonGroupItemsArranger::arrange_with_options(&items, 66., 3, 0., false);
    assert_eq!(r.items[0].column, 0);
    assert_eq!(r.items[0].height, 66.);
    assert_eq!(r.items[1].column, 1);
    assert_eq!(r.items[3].row, 2);
    assert_eq!(r.items[4].column, 2);
    assert_eq!(r.items[1].x, 50.);
    assert_eq!(r.items[4].x, 120.);
    assert_eq!(r.width, 150.);
}
#[test]
fn large_item_breaks_small_column() {
    let items = [
        RibbonArrangeItem::new(40., 22., false),
        RibbonArrangeItem::new(50., 66., true),
        RibbonArrangeItem::new(40., 22., false),
    ];
    let r = RibbonGroupItemsArranger::arrange_with_options(&items, 66., 3, 0., false);
    assert_eq!(
        r.items.iter().map(|i| i.column).collect::<Vec<_>>(),
        [0, 1, 2]
    );
}
#[test]
fn two_row_layout_and_column_break() {
    let mut items = [RibbonArrangeItem::new(40., 30., false); 4];
    items[3].starts_new_column = true;
    let r = RibbonGroupItemsArranger::arrange_with_options(&items, 60., 2, 0., false);
    assert_eq!(
        r.items.iter().map(|i| i.column).collect::<Vec<_>>(),
        [0, 0, 1, 2]
    );
    assert_eq!(r.items[1].y, 30.);
}
#[test]
fn distribute_rows_centers_partial_columns() {
    let items = [RibbonArrangeItem::new(40., 22., false)];
    let a = RibbonGroupItemsArranger::arrange(&items, 66.);
    let b = RibbonGroupItemsArranger::arrange_with_options(&items, 66., 3, 2., true);
    assert_eq!(a.items[0].y, 0.);
    assert!(b.items[0].y > 0.);
}
fn group(widths: &[f64]) -> RibbonSimplifiedGroup {
    RibbonSimplifiedGroup::new(
        widths
            .iter()
            .map(|w| RibbonSimplifiedItem {
                width: *w,
                visibility: V::Auto,
            })
            .collect(),
    )
}
#[test]
fn everything_in_line_when_it_fits() {
    let r = RibbonSimplifiedLayout::compute(&[group(&[30., 30.]), group(&[30.])], 200., 32.);
    assert!(!r.has_overflow);
    assert!(r.in_line.iter().flatten().all(|v| *v));
}
#[test]
fn removes_from_the_end_of_the_rightmost_group() {
    let r = RibbonSimplifiedLayout::compute(&[group(&[30., 30.]), group(&[30., 30.])], 110., 20.);
    assert!(r.has_overflow);
    assert_eq!(r.in_line, [vec![true, true], vec![true, false]]);
    assert!(r.total_width <= 110.);
}
#[test]
fn pinned_items_overflow_last_and_overflow_items_never_in_line() {
    let g = RibbonSimplifiedGroup::new(
        [V::Pinned, V::Auto, V::Overflow]
            .map(|v| RibbonSimplifiedItem {
                width: 30.,
                visibility: v,
            })
            .to_vec(),
    );
    let r = RibbonSimplifiedLayout::compute(&[g], 60., 20.);
    assert_eq!(r.in_line, [vec![true, false, false]]);
    assert!(r.has_overflow);
}
#[test]
fn hidden_items_are_neither_in_line_nor_counted() {
    let g = RibbonSimplifiedGroup::new(
        [V::Hidden, V::Auto]
            .map(|v| RibbonSimplifiedItem {
                width: 30.,
                visibility: v,
            })
            .to_vec(),
    );
    let r = RibbonSimplifiedLayout::compute(&[g], 100., 20.);
    assert_eq!(r.in_line, [vec![false, true]]);
    assert!(!r.has_overflow);
    assert_eq!(r.total_width, 30.);
}
#[test]
fn size_definition_parsing() {
    use RibbonSizeDefinition as D;
    assert_eq!(
        D::large_medium_small(),
        D::parse("Large, Middle, Small").unwrap()
    );
    assert_eq!(D::always_large(), D::parse("large").unwrap());
    assert_eq!(
        D::new(
            RibbonItemSize::Large,
            RibbonItemSize::Small,
            RibbonItemSize::Small
        ),
        D::parse("Large,Small").unwrap()
    );
    assert!(D::try_parse(Some("Huge")).is_none());
    assert_eq!(
        D::large_medium_small().get_size(S::Medium),
        RibbonItemSize::Medium
    );
    assert_eq!(
        D::large_medium_small().get_size(S::Collapsed),
        RibbonItemSize::Large
    );
}
#[test]
fn layout_tolerates_missing_widths() {
    let r = RibbonAdaptiveLayout::compute(
        &[
            RibbonGroupLayoutInfo::new(vec![100., 80.]),
            g(100., 80., 40., 20.),
        ],
        120.,
    );
    assert_eq!(r.states.len(), 2);
}
#[test]
fn group_by_group_reduction_collapses_low_priority_groups_first() {
    let mut groups = [
        g(200., 150., 100., 50.),
        g(200., 150., 100., 50.),
        g(200., 150., 100., 50.),
    ];
    groups[2].reduction_order = 1;
    let r = RibbonAdaptiveLayout::compute_with_strategy(
        &groups,
        460.,
        0.,
        RibbonReductionStrategy::GroupByGroup,
    );
    assert!(r.fits);
    assert_eq!(r.states, [S::Large, S::Large, S::Collapsed]);
    let stepwise = RibbonAdaptiveLayout::compute(&groups, 460.);
    assert_eq!(stepwise.states[0], S::Medium);
}
#[test]
fn shared_width_identity_null_widths_and_unnamed_states() {
    let mut group = normal();
    let copy = group.clone();
    group.widths.as_ref().unwrap().borrow_mut()[0] = 73.;
    assert_eq!(copy.get_width(S::Large), 73.);
    assert!(group.get_width(S::from_raw(99)).is_nan());
    group.widths = None;
    assert!(group.get_width(S::Large).is_nan());
}
#[test]
fn half_pixel_branch_preserves_source_false_fits() {
    let mut group = RibbonGroupLayoutInfo::new(vec![100., 99.75, 99.5]);
    group.can_collapse = false;
    let r = RibbonAdaptiveLayout::compute(&[group], 99.75);
    assert_eq!(r.states, [S::Small]);
    assert_eq!(r.total_width, 99.5);
    assert!(!r.fits);
}
#[test]
fn repeated_resizing_does_not_retain_previous_states() {
    let groups = [normal(), normal()];
    for width in [500., 10., 185., 145., 200., 0., 200.] {
        let r = RibbonAdaptiveLayout::compute(&groups, width);
        let fresh = RibbonAdaptiveLayout::compute(&[normal(), normal()], width);
        assert_eq!(r.states, fresh.states);
        assert_eq!(r.total_width, fresh.total_width);
        assert_eq!(r.fits, fresh.fits);
    }
}
