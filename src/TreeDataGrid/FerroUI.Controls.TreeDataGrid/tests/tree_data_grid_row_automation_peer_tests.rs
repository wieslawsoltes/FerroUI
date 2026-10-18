//! `TreeDataGridRowAutomationPeerTests.cs`: the peer of a row, the toggle and expand/collapse
//! providers it exposes only for rows that show their expander, its read-only value and the
//! selection of its row.
//!
//! The reference realizes the rows over a test implementation of the classic `IRows`; the rows of
//! the control here are `ITreeDataGridRows` over the core's rows (`IRow`, `IExpander`), which the
//! test rows implement.

use super::automation_peer_test_helper::{create_flat_target, create_peer, FlatTargetOptions};
use super::support::{assert_panics, run_jobs, start_app};
use crate::automation::peers::TreeDataGridRowAutomationPeer;
use crate::models::{CellRef, ColumnRef, ITreeDataGridRows};
use crate::primitives::TreeDataGridRow;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, ISelectionItemProvider, IToggleProvider, IValueProvider, ToggleState,
};
use ferroui_controls::automation::{ElementNotEnabledException, ExpandCollapseState};
use ferroui_treedatagrid_core::models::{IExpander, IRow, RowRef, RowsChangedHandler};
use ferroui_treedatagrid_core::{GridLength, ITreeDataGridSource, IndexPath};
use std::any::Any;
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

#[test]
fn should_create_tree_item_peer() {
    let _app = start_app();
    let row = TreeDataGridRow::new();
    let peer = create_peer::<TreeDataGridRowAutomationPeer>(&row);

    assert_eq!(AutomationControlType::TreeItem, peer.get_automation_control_type());
    assert!(peer.is_control_element());
    assert!(peer.is_content_element());
}

#[test]
fn should_expose_toggle_only_for_visible_expanders() {
    let _app = start_app();
    let non_expander_row = create_realized_row(TestRows::new(vec![TestRow::new(Some(new_object())) as RowRef]));
    let no_expander_peer = create_peer::<TreeDataGridRowAutomationPeer>(&non_expander_row);

    let hidden_expander_row = TestExpanderRow::new(Some(new_object()), false, false);
    let hidden_expander_owner = create_realized_row(TestRows::new(vec![hidden_expander_row.clone() as RowRef]));
    let hidden_expander_peer = create_peer::<TreeDataGridRowAutomationPeer>(&hidden_expander_owner);

    let expander_row = TestExpanderRow::new(Some(new_object()), true, false);
    let expandable_row = create_realized_row(TestRows::new(vec![expander_row.clone() as RowRef]));
    let expandable_peer = create_peer::<TreeDataGridRowAutomationPeer>(&expandable_row);
    let toggle_provider = expandable_peer.get_provider::<dyn IToggleProvider>().expect("a toggle provider");

    assert!(no_expander_peer.get_provider::<dyn IToggleProvider>().is_none());
    assert!(hidden_expander_peer.get_provider::<dyn IToggleProvider>().is_none());
    assert_eq!(ToggleState::Off, toggle_provider.toggle_state());

    toggle_provider.toggle().expect("the row is enabled");

    assert!(expander_row.is_expanded());
    assert_eq!(ToggleState::On, toggle_provider.toggle_state());
}

#[test]
fn should_expose_expand_collapse_only_for_visible_expanders() {
    let _app = start_app();
    let non_expander_row = create_realized_row(TestRows::new(vec![TestRow::new(Some(new_object())) as RowRef]));
    let non_expander_peer = create_peer::<TreeDataGridRowAutomationPeer>(&non_expander_row);

    let expander_row = TestExpanderRow::new(Some(new_object()), true, false);
    let expandable_row = create_realized_row(TestRows::new(vec![expander_row.clone() as RowRef]));
    let expandable_peer = create_peer::<TreeDataGridRowAutomationPeer>(&expandable_row);
    let provider =
        expandable_peer.get_provider::<dyn IExpandCollapseProvider>().expect("an expand/collapse provider");

    assert!(non_expander_peer.get_provider::<dyn IExpandCollapseProvider>().is_none());
    assert_eq!(ExpandCollapseState::Collapsed, provider.expand_collapse_state());

    provider.expand().expect("the row is enabled");
    assert!(expander_row.is_expanded());
    assert_eq!(ExpandCollapseState::Expanded, provider.expand_collapse_state());

    provider.collapse().expect("the row is enabled");
    assert!(!expander_row.is_expanded());
    assert_eq!(ExpandCollapseState::Collapsed, provider.expand_collapse_state());
}

#[test]
fn toggle_should_throw_when_row_is_disabled() {
    let _app = start_app();
    let row = create_realized_row(TestRows::new(vec![TestExpanderRow::new(Some(new_object()), true, false) as RowRef]));
    row.set_is_enabled(false);
    let peer = create_peer::<TreeDataGridRowAutomationPeer>(&row);
    let toggle_provider = peer.get_provider::<dyn IToggleProvider>().expect("a toggle provider");

    // `Assert.Throws<ElementNotEnabledException>`: the error of the provider's result.
    let result: Result<(), ElementNotEnabledException> = toggle_provider.toggle();
    assert!(result.is_err());
}

#[test]
fn value_provider_should_be_read_only() {
    let _app = start_app();
    // `TestRowModel.ToString()`: the display form of the model's type.
    ValueTypes::register_display::<TestRowModel>();
    let row = create_realized_row(TestRows::new(vec![
        TestRow::new(Some(Rc::new(TestRowModel::new("Node A")) as BoxedValue)) as RowRef,
    ]));
    let peer = create_peer::<TreeDataGridRowAutomationPeer>(&row);
    let value_provider = peer.get_provider::<dyn IValueProvider>().expect("a value provider");

    assert!(value_provider.is_read_only());
    assert_eq!(Some("Node A".to_string()), value_provider.value());
    // `NotSupportedException`: the peer panics.
    assert_panics(|| {
        let _ = value_provider.set_value(Some("Updated"));
    });
}

#[test]
fn selection_item_provider_should_select_and_deselect_row() {
    let _app = start_app();
    let (target, source, _items, root) =
        create_flat_target(FlatTargetOptions { single_select: false, ..FlatTargetOptions::default() });
    let row = target.try_get_row(0).expect("a realized row");
    let peer = create_peer::<TreeDataGridRowAutomationPeer>(&row);
    let selection_provider =
        peer.get_provider::<dyn ISelectionItemProvider>().expect("a selection item provider");
    let model_index = source.rows().row_index_to_model_index(row.row_index());
    let selection_model = target.row_selection().expect("a row selection model");

    assert!(selection_provider.selection_container().is_some());
    assert!(!selection_provider.is_selected());

    selection_provider.select().expect("the row is enabled");
    root.update_layout();
    run_jobs();
    assert!(selection_model.is_selected(&model_index));
    assert!(selection_provider.is_selected());

    selection_provider.remove_from_selection().expect("the row is enabled");
    root.update_layout();
    run_jobs();
    assert!(!selection_model.is_selected(&model_index));
    assert!(!selection_provider.is_selected());

    selection_provider.add_to_selection().expect("the row is enabled");
    root.update_layout();
    run_jobs();
    assert!(selection_model.is_selected(&model_index));
    assert!(selection_provider.is_selected());

    root.close();
}

/// `CreateRealizedRow(rows)`: a row realized for the first row of `rows`, without an element
/// factory, a selection or columns.
fn create_realized_row(rows: Rc<TestRows>) -> Ref<TreeDataGridRow> {
    let row = TreeDataGridRow::new();
    let rows = rows as Rc<dyn ITreeDataGridRows>;
    row.realize(None, None, None, Some(&rows), 0);
    row
}

/// `new object()`: a model with nothing but its identity.
fn new_object() -> BoxedValue {
    Rc::new(TestObject) as BoxedValue
}

struct TestObject;

impl PartialEq for TestObject {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `TestRows`: fixed rows; their model indexes are never found and their cells are never
/// realized.
struct TestRows {
    rows: Vec<RowRef>,
}

impl TestRows {
    fn new(rows: Vec<RowRef>) -> Rc<Self> {
        Rc::new(Self { rows })
    }
}

impl ITreeDataGridRows for TestRows {
    fn count(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, index: usize) -> RowRef {
        self.rows[index].clone()
    }

    fn get_row_at(&self, _y: f64) -> (i32, f64) {
        (0, 0.0)
    }

    fn model_index_to_row_index(&self, _model_index: &IndexPath) -> i32 {
        -1
    }

    fn row_index_to_model_index(&self, _row_index: i32) -> IndexPath {
        IndexPath::default()
    }

    /// # Panics
    /// Always (`NotSupportedException`).
    fn realize_cell(&self, _column: &ColumnRef, _column_index: i32, _row_index: i32) -> CellRef {
        panic!("The test rows do not realize cells.");
    }

    fn unrealize_cell(&self, _cell: &CellRef, _column_index: i32, _row_index: i32) {}

    // `CollectionChanged { add { } remove { } }`: the rows never change.
    fn add_collection_changed(&self, _handler: Rc<RowsChangedHandler>) -> u64 {
        0
    }

    fn remove_collection_changed(&self, _token: u64) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `TestRow`: a row of a model, without a header.
struct TestRow {
    model: Option<BoxedValue>,
    height: Cell<GridLength>,
}

impl TestRow {
    fn create(model: Option<BoxedValue>) -> Self {
        Self { model, height: Cell::new(GridLength::auto()) }
    }

    fn new(model: Option<BoxedValue>) -> Rc<Self> {
        Rc::new(Self::create(model))
    }
}

impl IRow for TestRow {
    fn header(&self) -> Option<BoxedValue> {
        None
    }

    fn height(&self) -> GridLength {
        self.height.get()
    }

    fn set_height(&self, value: GridLength) {
        self.height.set(value);
    }

    fn model(&self) -> Option<BoxedValue> {
        self.model.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `TestExpanderRow`: a test row that is an expander.
struct TestExpanderRow {
    base: TestRow,
    is_expanded: Cell<bool>,
    show_expander: Cell<bool>,
}

impl TestExpanderRow {
    fn new(model: Option<BoxedValue>, show_expander: bool, is_expanded: bool) -> Rc<Self> {
        Rc::new(Self {
            base: TestRow::create(model),
            is_expanded: Cell::new(is_expanded),
            show_expander: Cell::new(show_expander),
        })
    }
}

impl IRow for TestExpanderRow {
    fn header(&self) -> Option<BoxedValue> {
        self.base.header()
    }

    fn height(&self) -> GridLength {
        self.base.height()
    }

    fn set_height(&self, value: GridLength) {
        self.base.set_height(value);
    }

    fn model(&self) -> Option<BoxedValue> {
        self.base.model()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_expander(&self) -> Option<&dyn IExpander> {
        Some(self)
    }
}

impl IExpander for TestExpanderRow {
    fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    fn set_is_expanded(&self, value: bool) {
        self.is_expanded.set(value);
    }

    fn show_expander(&self) -> bool {
        self.show_expander.get()
    }
}

/// `TestRowModel`: a model whose text is its name (`ToString()`).
struct TestRowModel {
    name: String,
}

impl TestRowModel {
    fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

impl PartialEq for TestRowModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Display for TestRowModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}
