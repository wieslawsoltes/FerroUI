//! `CoreCellRecyclingTests.cs`: the pool of the cells of a presentation of a core source: a pooled
//! cell releases the subscriptions of its row and is retargeted to another row, pools are kept per
//! column and per view, cleared when a column is hidden or the view suspended, bounded, and do not
//! keep removed models alive; horizontal scrolling reuses the cells of the grid.

use super::support::{
    observable, presentation_of, run_jobs, source_ref, start_app, test_window, tree_data_grid_row_style,
    tree_data_grid_template, TestNotifier,
};
use crate::models::{CellRef, CheckBoxCell, ICell, IColumns, ITextCell, ITreeDataGridRows};
use crate::presentation::{CellModelPool, TreeDataGridPresentation, TreeDataGridSourcePresentation};
use crate::primitives::{TreeDataGridCell, TreeDataGridRow};
use crate::TreeDataGrid;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{ferro_model, BoxedValue, Size, Vector};
use ferroui_treedatagrid_core::models::{self as core_models, ColumnRef as CoreColumnRef, IColumn as CoreColumn};
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, GridLength as CoreGridLength, ITreeDataGridSource, Models};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

// ---------------------------------------------------------------------------------------------
// Models and helpers
// ---------------------------------------------------------------------------------------------

/// `Node`: the state the cells bind below the node, and a constant `Other`.
struct Node {
    state: Rc<State>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Node {
    fn new(name: &str) -> Rc<Self> {
        let state = State::new();
        state.set_name(name.to_string());
        Rc::new(Self { state })
    }

    fn other(&self) -> String {
        "Other".to_string()
    }
}

/// `State`: a name and a flag that notify their changes, and the number of subscribers to the
/// notifications (`SubscriberCount`).
struct State {
    notifier: TestNotifier,
    name: RefCell<String>,
    enabled: Cell<bool>,
}

impl State {
    fn new() -> Rc<Self> {
        Model::new_model(Self { notifier: TestNotifier::new(), name: RefCell::new(String::new()), enabled: Cell::new(false) })
    }

    fn subscriber_count(&self) -> usize {
        self.notifier.property_changed_subscriber_count()
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: String) {
        *self.name.borrow_mut() = value;
        self.notifier.raise("Name");
    }

    fn enabled(&self) -> bool {
        self.enabled.get()
    }

    fn set_enabled(&self, value: bool) {
        self.enabled.set(value);
        self.notifier.raise("Enabled");
    }
}

impl INotifyPropertyChanged for State {
    fn property_changed(&self) -> &Event<str> {
        self.notifier.event()
    }
}

ferro_model!(State, |b| b.notify_property_changed());

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// Whether two shared values are the same object (`Assert.Same`).
fn same<A: ?Sized, B: ?Sized>(a: &Rc<A>, b: &Rc<B>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// `x => x.State.Name`: the member `Name` of the node's state, observed below the node.
fn state_name() -> Accessor<Node, String> {
    Accessor::member("Name", |x: &Rc<Node>| x.state.name()).through(|x: &Rc<Node>| Some(x.state.clone() as BoxedValue), "Name")
}

/// `x => x.Other`.
fn other() -> Accessor<Node, String> {
    Accessor::member("Other", |x: &Rc<Node>| x.other())
}

fn pixels(value: f64) -> Option<CoreGridLength> {
    Some(CoreGridLength::pixels(value))
}

/// `Source(items)`: a flat source of `items` with a writable column of the state's name and a
/// column of `Other`, 100 pixels wide each.
fn source(items: Vec<Rc<Node>>) -> Rc<FlatTreeDataGridSource<Node>> {
    let source = FlatTreeDataGridSource::new(Rc::new(items) as Models<Node>);
    source.columns().add(core_models::TextColumn::new(
        header("Name"),
        state_name().with_setter(|x: &Rc<Node>, value: String| x.state.set_name(value)),
        pixels(100.0),
        None,
        None,
    ) as CoreColumnRef<Node>);
    source.columns().add(core_models::TextColumn::new(header("Other"), other(), pixels(100.0), None, None) as CoreColumnRef<Node>);
    source
}

fn text(cell: &CellRef) -> Option<String> {
    cell.as_text_cell().expect("a text cell").text()
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[test]
fn pooled_binding_releases_nested_subscriptions_and_retargets_edits() {
    let _app = start_app();
    let first = Node::new("First");
    let second = Node::new("Second");
    let source = source(vec![first.clone(), second.clone()]);
    let view = presentation_of::<Node, _>(&source);
    let rows = view.rows();
    let column = view.columns().get(0);

    let cell = rows.realize_cell(&column, 0, 0);
    assert!(first.state.subscriber_count() > 0);
    assert!(rows.try_recycle_cell(&column, &cell));
    assert_eq!(0, first.state.subscriber_count());
    assert!(cell.value().is_none());

    first.state.set_name("Old row changed".to_string());
    assert!(cell.value().is_none());
    assert!(same(&cell, &rows.realize_cell(&column, 0, 1)));
    assert_eq!(Some("Second".to_string()), text(&cell));
    second.state.set_name("Updated".to_string());
    assert_eq!(Some("Updated".to_string()), text(&cell));
    cell.as_text_cell().expect("a text cell").set_text(Some("Edited".to_string()));
    assert_eq!("Edited", second.state.name());
    assert_eq!("Old row changed", first.state.name());
    rows.unrealize_cell(&cell, 0, 1);
    assert_eq!(0, second.state.subscriber_count());
}

#[test]
fn checkboxes_release_subscriptions_and_retarget() {
    let _app = start_app();
    let first = Node::new("First");
    let second = Node::new("Second");
    second.state.set_enabled(true);
    let source = source(vec![first.clone(), second.clone()]);
    source.columns().add(core_models::CheckBoxColumn::new(
        header("Enabled"),
        Accessor::member("Enabled", |x: &Rc<Node>| x.state.enabled())
            .through(|x: &Rc<Node>| Some(x.state.clone() as BoxedValue), "Enabled")
            .with_setter(|x: &Rc<Node>, value: bool| x.state.set_enabled(value)),
        None,
        None,
        None,
    ) as CoreColumnRef<Node>);
    let view = presentation_of::<Node, _>(&source);
    let rows = view.rows();
    let column = view.columns().get(2);

    let cell = rows.realize_cell(&column, 2, 0);
    let check_box = cell.as_any().downcast_ref::<CheckBoxCell>().expect("a check box cell");
    assert!(rows.try_recycle_cell(&column, &cell));
    assert_eq!(0, first.state.subscriber_count());
    assert!(same(&cell, &rows.realize_cell(&column, 2, 1)));
    assert_eq!(Some(true), check_box.value());
    check_box.set_value(Some(false));
    assert!(!second.state.enabled());
    rows.unrealize_cell(&cell, 2, 1);
}

#[test]
fn pools_do_not_mix_column_bindings_or_views() {
    let _app = start_app();
    let source = source(vec![Node::new("First")]);
    let first_view = presentation_of::<Node, _>(&source);
    let second_view = presentation_of::<Node, _>(&source);
    let first_rows = first_view.rows();
    let second_rows = second_view.rows();

    let cell = first_rows.realize_cell(&first_view.columns().get(0), 0, 0);
    assert!(first_rows.try_recycle_cell(&first_view.columns().get(0), &cell));
    let other_column = first_rows.realize_cell(&first_view.columns().get(1), 1, 0);
    let other_view = second_rows.realize_cell(&second_view.columns().get(0), 0, 0);
    assert!(!same(&cell, &other_column));
    assert_eq!(Some("Other".to_string()), text(&other_column));
    assert!(!same(&cell, &other_view));
    first_rows.unrealize_cell(&other_column, 1, 0);
    second_rows.unrealize_cell(&other_view, 0, 0);
    assert!(same(&cell, &first_rows.realize_cell(&first_view.columns().get(0), 0, 0)));
    first_rows.unrealize_cell(&cell, 0, 0);
}

fn hiding_columns_and_suspending_views_clear_pools(suspend: bool) {
    let _app = start_app();
    let source = source(vec![Node::new("First")]);
    let view = presentation_of::<Node, _>(&source);
    let rows = view.rows();
    let column = view.columns().get(0);
    let cell = rows.realize_cell(&column, 0, 0);
    assert!(rows.try_recycle_cell(&column, &cell));

    if suspend {
        view.suspend();
        view.resume();
    } else {
        source.columns().get(0).set_is_visible(false);
        source.columns().get(0).set_is_visible(true);
    }

    let replacement = rows.realize_cell(&view.columns().get(0), 0, 0);
    assert!(!same(&cell, &replacement));
    rows.unrealize_cell(&replacement, 0, 0);
}

#[test]
fn hiding_columns_and_suspending_views_clear_pools_hide() {
    hiding_columns_and_suspending_views_clear_pools(false);
}

#[test]
fn hiding_columns_and_suspending_views_clear_pools_suspend() {
    hiding_columns_and_suspending_views_clear_pools(true);
}

#[test]
fn pool_evicts_models_when_its_budget_is_exceeded() {
    let _app = start_app();
    let source = source(vec![Node::new("First")]);
    let view = presentation_of::<Node, _>(&source);
    let rows = view.rows();
    let column = view.columns().get(0);

    let cells: Vec<CellRef> = (0..CellModelPool::MAXIMUM_CELLS + 1).map(|_| rows.realize_cell(&column, 0, 0)).collect();
    for cell in &cells {
        assert!(rows.try_recycle_cell(&column, cell));
    }
    let last = cells.last().expect("a cell");
    assert!(same(last, &rows.realize_cell(&column, 0, 0)));
    let replacement = rows.realize_cell(&column, 0, 0);
    assert!(!cells.iter().any(|cell| same(cell, &replacement)));
    rows.unrealize_cell(last, 0, 0);
    rows.unrealize_cell(&replacement, 0, 0);
}

/// `CreatePoolWithRemovedRow`: a view whose pool holds the cell of a model that was removed from
/// the source, and a weak reference to the model.
fn create_pool_with_removed_row() -> (Rc<TreeDataGridSourcePresentation>, Weak<Node>) {
    let original = Node::new("Original");
    let items = observable(vec![original.clone(), Node::new("Replacement")]);
    let source = source(vec![original.clone(), items.get(1)]);
    source.set_items(items.clone() as Models<Node>);
    let view = presentation_of::<Node, _>(&source);
    let rows = view.rows();
    let column = view.columns().get(0);
    let cell = rows.realize_cell(&column, 0, 0);
    assert!(rows.try_recycle_cell(&column, &cell));
    items.remove_at(0);
    // Flat Core rows reuse a row cursor; move it away from the removed item too.
    let _ = ITreeDataGridSource::rows(&*source).get(0);
    (view, Rc::downgrade(&original))
}

#[test]
fn pooled_cells_do_not_retain_the_original_row_or_value() {
    let _app = start_app();
    let (view, weak_row) = create_pool_with_removed_row();
    // The reference collects the garbage and asserts that the model is no longer alive while the
    // view is: here every strong reference the test made is gone, so a model kept by the pool (its
    // cell, the cell's binding or the row) would still be alive.
    assert!(weak_row.upgrade().is_none());
    drop(view);
}

fn horizontal_scrolling_reuses_models_and_detach_releases_subscriptions_core(insert_column: bool) {
    let _app = start_app();
    let item = Node::new("First");
    let source = source(vec![item.clone()]);
    for _ in 2..20 {
        source.columns().add(core_models::TextColumn::new(header("More"), state_name(), pixels(100.0), None, None) as CoreColumnRef<Node>);
    }
    let grid = TreeDataGrid::new();
    grid.set_model(source_ref::<Node, _>(&source));
    grid.set_template(Some(tree_data_grid_template(0.0)));
    let window = test_window(Some(grid.clone().upcast()), Some(Size::new(200.0, 100.0)));
    window.styles().add(tree_data_grid_row_style());
    window.styles().add(Style::with_setters(
        Selectors::of_type::<TreeDataGridRow>(),
        [Setter::new(Layoutable::height_property(), 20.0)],
    ));

    window.update_layout();
    run_jobs();
    let row = grid
        .rows_presenter()
        .expect("a rows presenter")
        .try_get_element(0)
        .and_then(|element| element.cast::<TreeDataGridRow>())
        .expect("a realized row");
    let cells_presenter = row.cells_presenter().expect("a cells presenter");
    let original = cells_presenter
        .try_get_element(0)
        .and_then(|element| element.cast::<TreeDataGridCell>())
        .expect("a realized cell")
        .model()
        .expect("a cell model");
    if insert_column {
        source.columns().insert(
            0,
            core_models::TextColumn::new(header("Inserted"), other(), pixels(100.0), None, None) as CoreColumnRef<Node>,
        );
        window.update_layout();
    }

    let scroll = grid.scroll().expect("a scroll viewer");
    scroll.set_offset(Vector::new(300.0, 0.0));
    window.update_layout();
    let original_index = if insert_column { 1 } else { 0 };
    let cells_presenter = row.cells_presenter().expect("a cells presenter");
    assert!(cells_presenter.try_get_element(original_index).is_none());

    scroll.set_offset(Vector::new(f64::from(original_index) * 100.0, 0.0));
    window.update_layout();
    let rebound = cells_presenter
        .try_get_element(original_index)
        .and_then(|element| element.cast::<TreeDataGridCell>())
        .expect("a rebound cell");
    let rebound_model = rebound.model().expect("a cell model");
    assert!(same(&original, &rebound_model));
    assert_eq!(Some("First".to_string()), text(&rebound_model));

    window.set_content(None);
    window.update_layout();
    assert_eq!(0, item.state.subscriber_count());

    window.close();
}

#[test]
fn horizontal_scrolling_reuses_models_and_detach_releases_subscriptions() {
    horizontal_scrolling_reuses_models_and_detach_releases_subscriptions_core(false);
}

#[test]
fn horizontal_scrolling_reuses_models_and_detach_releases_subscriptions_after_insert() {
    horizontal_scrolling_reuses_models_and_detach_releases_subscriptions_core(true);
}
