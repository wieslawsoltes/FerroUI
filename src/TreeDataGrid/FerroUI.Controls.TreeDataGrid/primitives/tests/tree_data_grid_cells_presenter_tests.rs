//! `TreeDataGridCellsPresenterTests.cs`.
//!
//! The reference's rows are an array of null models; a model here is a value, so the rows hold
//! default models.

use super::support::{
    column_list, has_logical_child, has_visual_child, is_red, layout, layout_test_columns, red_brush, rows_of_array,
    run_jobs, scroll_viewer, start_app, test_element_factory, test_window, visual_children_of, LayoutTestCellControl,
    LayoutTestColumn, TreeAttachmentCounter,
};
use crate::models::{CellRef, ColumnListBase, ColumnOptions, ColumnRef, IColumns, ITreeDataGridRows};
use crate::primitives::{TreeDataGridCell, TreeDataGridCellsPresenter, TreeDataGridCellsPresenterImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref, Size, StyledElementImpl, Vector, VisualImpl};
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControl};
use ferroui_controls::{Control, ControlImpl, GridLength, GridUnitType, ScrollViewer, Window};
use std::cell::Cell;
use std::rc::Rc;

struct Model {
    #[allow(dead_code)]
    id: i32,
    #[allow(dead_code)]
    title: Option<String>,
}

impl PartialEq for Model {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `CountingCellsPresenter`: counts the measures of the presenter.
#[repr(C)]
struct CountingCellsPresenter {
    base: TreeDataGridCellsPresenter,
    measure_count: Cell<i32>,
}

ferro_class!(CountingCellsPresenter: TreeDataGridCellsPresenter);
ferroui_base::ferro_impl_classes!(
    CountingCellsPresenter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TreeDataGridCellsPresenterImpl
);

impl LayoutableImpl for CountingCellsPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measure_count.set(this.measure_count.get() + 1);
        Self::parent_measure_override(this, available_size)
    }
}

impl CountingCellsPresenter {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TreeDataGridCellsPresenter::construct(), measure_count: Cell::new(0) })
    }
}

struct Target {
    target: Ref<TreeDataGridCellsPresenter>,
    scroll: Ref<ScrollViewer>,
    columns: Rc<ColumnListBase>,
    _root: Ref<Window>,
    _viewport_subscription: Rc<dyn IDisposable>,
}

fn create_target(
    columns: Option<Rc<ColumnListBase>>,
    additional_styles: Vec<Ref<Style>>,
    row_count: usize,
    presenter: Option<Ref<TreeDataGridCellsPresenter>>,
) -> Target {
    let columns = columns.unwrap_or_else(|| layout_test_columns::<Model>(100));

    let items: Vec<Rc<Model>> = (0..row_count).map(|_| Rc::new(Model { id: 0, title: None })).collect();
    let rows = rows_of_array(items);

    let target = presenter.unwrap_or_else(TreeDataGridCellsPresenter::new);
    target.set_element_factory(Some(test_element_factory()));
    target.set_items(Some(columns.clone() as Rc<dyn IColumns>));
    target.set_rows(Some(rows as Rc<dyn ITreeDataGridRows>));

    // The column list's effective viewport would usually be updated by the rows presenter but in
    // this case we don't have one, so do it manually.
    let viewport_columns = columns.clone();
    let viewport_subscription =
        target.effective_viewport_changed(move |e| viewport_columns.viewport_changed(e.effective_viewport()));

    target.realize(0);

    let scroll = scroll_viewer(target.clone().upcast());
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);

    let root = test_window(Some(scroll.clone().upcast()), None);

    for style in additional_styles {
        root.styles().add(style);
    }

    root.update_layout();
    run_jobs();

    Target { target, scroll, columns, _root: root, _viewport_subscription: viewport_subscription }
}

fn default_target() -> Target {
    create_target(None, Vec::new(), 1, None)
}

fn cells(target: &TreeDataGridCellsPresenter) -> Vec<Ref<TreeDataGridCell>> {
    target.realized_elements().into_iter().flatten().map(|element| element.cast::<TreeDataGridCell>().expect("a cell")).collect()
}

fn assert_column_indexes(target: &TreeDataGridCellsPresenter, first_column_index: i32, column_count: i32) {
    let mut indexes: Vec<i32> = visual_children_of::<TreeDataGridCell>(target)
        .iter()
        .filter(|x| x.is_visible())
        .map(|x| x.column_index())
        .collect();
    indexes.sort();

    assert_eq!((first_column_index..first_column_index + column_count).collect::<Vec<i32>>(), indexes);
}

fn assert_recyclable(target: &TreeDataGridCellsPresenter, count: usize) {
    let recyclable = visual_children_of::<TreeDataGridCell>(target).into_iter().filter(|x| !x.is_visible()).count();
    assert_eq!(count, recyclable);
}

fn same_cell(a: &Option<CellRef>, b: &Option<CellRef>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

fn layout_test_cell_control(element: &Option<Ref<Control>>) -> Ref<LayoutTestCellControl> {
    element.as_ref().and_then(|element| element.cast::<LayoutTestCellControl>()).expect("a layout test cell control")
}

#[test]
fn creates_initial_cells() {
    let _app = start_app();
    let t = default_target();

    assert_column_indexes(&t.target, 0, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn updates_column_actual_width() {
    let _app = start_app();
    let t = default_target();

    for i in 0..t.columns.count() {
        let column = t.columns.get(i);
        assert_eq!(if i < 10 { 10.0 } else { 0.0 }, column.actual_width());
    }
}

#[test]
fn scrolls_right_one_cell() {
    let _app = start_app();
    let t = default_target();

    t.scroll.set_offset(Vector::new(10.0, 0.0));
    layout(&t.target);

    assert_column_indexes(&t.target, 1, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn full_cell_scroll_reuses_controls_without_tree_reattachment() {
    let _app = start_app();
    let t = default_target();
    let realized = cells(&t.target);
    let counter = TreeAttachmentCounter::new();

    for cell in &realized {
        counter.watch(cell);
    }

    t.scroll.set_offset(Vector::new(10.0, 0.0));
    layout(&t.target);

    let mut after = cells(&t.target);
    let mut before = realized.clone();
    before.sort_by_key(|cell| cell.column_index());
    after.sort_by_key(|cell| cell.column_index());
    assert_eq!(before.len(), after.len());
    assert!(before.iter().all(|cell| after.contains(cell)));
    counter.assert_none();
    assert_column_indexes(&t.target, 1, 10);
}

#[test]
fn small_scrolls_within_realized_cells_do_not_repeat_measure() {
    let _app = start_app();
    let presenter = CountingCellsPresenter::new();
    let t = create_target(None, Vec::new(), 1, Some(presenter.clone().upcast()));

    t.scroll.set_offset(Vector::new(1.0, 0.0));
    layout(&t.target);
    let measure_count_after_realizing_trailing_cell = presenter.measure_count.get();

    for offset in 2..=10 {
        t.scroll.set_offset(Vector::new(f64::from(offset), 0.0));
        layout(&t.target);
    }

    assert_eq!(measure_count_after_realizing_trailing_cell, presenter.measure_count.get());
    assert_column_indexes(&t.target, 0, 11);

    t.scroll.set_offset(Vector::new(11.0, 0.0));
    layout(&t.target);

    assert!(presenter.measure_count.get() > measure_count_after_realizing_trailing_cell);
    assert_column_indexes(&t.target, 1, 11);
}

#[test]
fn scrolls_right_more_than_a_page() {
    let _app = start_app();
    let t = default_target();

    t.scroll.set_offset(Vector::new(200.0, 0.0));
    layout(&t.target);

    assert_column_indexes(&t.target, 20, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn scrolls_left_more_than_a_page() {
    let _app = start_app();
    let t = default_target();

    t.scroll.set_offset(Vector::new(200.0, 0.0));
    layout(&t.target);

    t.scroll.set_offset(Vector::new(0.0, 0.0));
    layout(&t.target);

    assert_column_indexes(&t.target, 0, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn reuses_realized_cell_controls_when_row_is_immediately_rebound() {
    let _app = start_app();
    let t = create_target(None, Vec::new(), 2, None);
    let realized = t.target.realized_elements();
    let realized_cells = cells(&t.target);
    let models: Vec<Option<CellRef>> = realized_cells.iter().map(|cell| cell.model()).collect();

    t.target.unrealize();

    assert_eq!(realized, t.target.realized_elements());
    assert!(realized_cells.iter().all(|cell| cell.row_index() == 0));

    t.target.realize(1);
    layout(&t.target);

    assert_eq!(realized, t.target.realized_elements());
    assert!(realized_cells.iter().all(|cell| cell.row_index() == 1));
    assert!(realized_cells.iter().zip(models.iter()).all(|(cell, model)| !same_cell(model, &cell.model())));
}

#[test]
fn deferred_row_cells_are_recycled_without_tree_reattachment_when_finalized() {
    let _app = start_app();
    let t = create_target(None, Vec::new(), 2, None);
    let realized_cells = cells(&t.target);
    let counter = TreeAttachmentCounter::new();

    for cell in &realized_cells {
        counter.watch(cell);
    }

    t.target.unrealize();
    t.target.finalize_unrealize();

    assert!(t.target.realized_elements().is_empty());
    for cell in &realized_cells {
        assert_eq!(-1, cell.column_index());
        assert_eq!(-1, cell.row_index());
        assert!(cell.model().is_none());
        assert!(!cell.is_visible());
        assert!(has_logical_child(&t.target, cell));
        assert!(has_visual_child(&t.target, cell));
    }

    t.target.realize(1);
    layout(&t.target);

    let after = cells(&t.target);
    assert_eq!(realized_cells.len(), after.len());
    assert!(realized_cells.iter().all(|cell| after.contains(cell)));
    counter.assert_none();
}

#[test]
fn deferred_row_cells_are_recycled_when_presenter_detaches() {
    let _app = start_app();
    let t = create_target(None, Vec::new(), 2, None);
    let realized_cells = cells(&t.target);

    t.target.unrealize();
    t.scroll.set_content(None);
    t.scroll.update_layout();
    run_jobs();

    assert!(t.target.realized_elements().is_empty());
    for cell in &realized_cells {
        assert_eq!(-1, cell.column_index());
        assert_eq!(-1, cell.row_index());
        assert!(cell.model().is_none());
    }
}

#[test]
fn deferred_row_cells_are_recycled_before_rows_source_changes() {
    let _app = start_app();
    let t = create_target(None, Vec::new(), 2, None);
    let realized_cells = cells(&t.target);

    t.target.unrealize();
    t.target.set_rows(None);

    assert!(t.target.realized_elements().is_empty());
    for cell in &realized_cells {
        assert_eq!(-1, cell.column_index());
        assert_eq!(-1, cell.row_index());
        assert!(cell.model().is_none());
    }
}

#[test]
fn moving_columns_preserves_realized_cell_identity() {
    let _app = start_app();
    let columns = layout_test_columns::<Model>(100);

    let t = create_target(Some(columns.clone()), Vec::new(), 1, None);
    let realized = t.target.realized_elements();

    columns.move_item(2, 7);
    layout(&t.target);

    assert_eq!(realized[0], t.target.try_get_element(0));
    assert_eq!(realized[1], t.target.try_get_element(1));
    for old_index in 3..=7 {
        assert_eq!(realized[old_index as usize], t.target.try_get_element(old_index - 1));
    }
    assert_eq!(realized[2], t.target.try_get_element(7));
    assert_eq!(realized[8], t.target.try_get_element(8));
    assert_eq!(realized[9], t.target.try_get_element(9));
    assert_column_indexes(&t.target, 0, 10);
}

fn min_width_options(min_width: f64) -> ColumnOptions<Model> {
    let options = ColumnOptions::new();
    options.set_min_width(GridLength::new(min_width, GridUnitType::Pixel));
    options
}

#[test]
fn desired_size_takes_min_star_column_width_into_account() {
    let _app = start_app();
    let columns = column_list([
        LayoutTestColumn::<Model>::new("Col0", Some(GridLength::STAR), Some(min_width_options(100.0))) as ColumnRef,
        LayoutTestColumn::<Model>::new("Col1", Some(GridLength::STAR), Some(min_width_options(100.0))) as ColumnRef,
    ]);

    let t = create_target(Some(columns), Vec::new(), 1, None);

    assert_eq!(200.0, t.target.desired_size().width);
}

#[test]
fn star_cells_are_measured_with_final_column_width() {
    // Issue #70 of the reference.
    let _app = start_app();
    let columns = column_list([
        LayoutTestColumn::<Model>::new("Col0", Some(GridLength::STAR), None) as ColumnRef,
        LayoutTestColumn::<Model>::new("Col1", Some(GridLength::STAR), None) as ColumnRef,
    ]);

    let t = create_target(Some(columns), Vec::new(), 1, None);

    for element in t.target.realized_elements() {
        let cell = layout_test_cell_control(&element);

        assert_eq!(
            vec![Size::INFINITY, Size::new(0.0, f64::INFINITY), Size::new(50.0, f64::INFINITY)],
            cell.measure_constraints()
        );
    }
}

#[test]
fn fixed_cells_are_measured_only_with_final_column_width() {
    let _app = start_app();
    let columns = column_list([
        LayoutTestColumn::<Model>::new("Col0", Some(GridLength::new(40.0, GridUnitType::Pixel)), None) as ColumnRef,
        LayoutTestColumn::<Model>::new("Col1", Some(GridLength::new(60.0, GridUnitType::Pixel)), None) as ColumnRef,
    ]);

    let t = create_target(Some(columns), Vec::new(), 1, None);
    let realized = t.target.realized_elements();

    assert_eq!(vec![Size::new(40.0, f64::INFINITY)], layout_test_cell_control(&realized[0]).measure_constraints());
    assert_eq!(vec![Size::new(60.0, f64::INFINITY)], layout_test_cell_control(&realized[1]).measure_constraints());
}

#[test]
fn auto_cell_natural_measure_cache_is_invalidated_with_the_cell() {
    let _app = start_app();
    let columns = column_list([LayoutTestColumn::<Model>::new("Col0", Some(GridLength::AUTO), None) as ColumnRef]);
    let t = create_target(Some(columns), Vec::new(), 1, None);
    let cell = layout_test_cell_control(&t.target.realized_elements()[0]);
    let initial_constraints = cell.measure_constraints();

    assert!(initial_constraints.contains(&Size::INFINITY));
    assert!(initial_constraints.contains(&Size::new(10.0, f64::INFINITY)));

    t.target.invalidate_measure();
    layout(&t.target);

    assert_eq!(initial_constraints, cell.measure_constraints());

    cell.invalidate_measure();
    t.target.invalidate_measure();
    layout(&t.target);

    let count_infinity = |constraints: &[Size]| constraints.iter().filter(|x| **x == Size::INFINITY).count();
    assert_eq!(count_infinity(&initial_constraints) + 1, count_infinity(&cell.measure_constraints()));
    assert_eq!(Some(&Size::new(10.0, f64::INFINITY)), cell.measure_constraints().last());
}

#[test]
fn rebinding_auto_cell_discards_natural_measure_cache() {
    let _app = start_app();
    let columns = column_list([LayoutTestColumn::<Model>::new("Col0", Some(GridLength::AUTO), None) as ColumnRef]);
    let t = create_target(Some(columns), Vec::new(), 2, None);
    let cell = layout_test_cell_control(&t.target.realized_elements()[0]);
    let count_infinity = |constraints: &[Size]| constraints.iter().filter(|x| **x == Size::INFINITY).count();
    let initial_natural_measures = count_infinity(&cell.measure_constraints());

    t.target.unrealize();
    t.target.realize(1);
    layout(&t.target);

    assert_eq!(initial_natural_measures + 1, count_infinity(&cell.measure_constraints()));
}

#[test]
fn nth_child_handles_deletion_and_addition_correctly() {
    let _app = start_app();
    let style = Style::with_setters(
        Selectors::of_type::<TreeDataGridCellsPresenter>().descendant().is::<TreeDataGridCell>().nth_child(2, 0),
        [Setter::new(TemplatedControl::background_property(), red_brush())],
    );
    let t = create_target(None, vec![style], 1, None);

    layout(&t.target);

    let red = visual_children_of::<TreeDataGridCell>(&t.target).into_iter().filter(|cell| is_red(cell.background())).count();
    assert_eq!(5, red);
}
