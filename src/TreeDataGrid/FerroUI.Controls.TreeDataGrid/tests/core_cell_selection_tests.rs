//! `CoreCellSelectionTests.cs`: the core's cell selection presented to the grid: the mapping of
//! sorted rows and hidden columns to visible cells, the interactions of replaced and disposed
//! selections, keyboard and pointer selection, native view columns whose measurement follows
//! their width, and the cell selection mode of the grid.

use super::support::{
    source_ref, start_app, test_element_factory, test_window, tree_data_grid_row_style, tree_data_grid_template,
    LayoutTestCell, LayoutTestCellControl,
};
use crate::models::{CellRef, ColumnRef, ICellColumn, IColumn, IUpdateColumnLayout};
use crate::presentation::{
    CellColumnBase, CellColumnOptions, ITreeDataGridPresentationOptions, TreeDataGridPresentation,
    TreeDataGridPresentationOptions, TreeDataGridSourcePresentation,
};
use crate::primitives::TreeDataGridCell;
use crate::{TreeDataGrid, TreeDataGridSelectionMode};
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::input::{
    IPointer, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, Pointer, PointerPointProperties,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::{BoxedValue, Point, Ref, Visual};
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::models::{
    self as core_models, ColumnRef as CoreColumnRef, IColumn as CoreColumn, ListSortDirection, RowRef,
};
use ferroui_treedatagrid_core::selection::{
    downcast_selection, ITreeDataGridSelection, TreeDataGridCellSelectionModel, TreeDataGridRowSelectionModel,
};
use ferroui_treedatagrid_core::{
    Accessor, CellIndex, FlatTreeDataGridSource, GridLength as CoreGridLength, ITreeDataGridSource, ITreeDataGridSourceOf,
    Models,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// Models and helpers
// ---------------------------------------------------------------------------------------------

/// `record Node(string Name)`: a name, compared by value as a record is.
#[derive(PartialEq)]
struct Node {
    name: String,
}

impl Node {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: name.to_string() })
    }
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// `x => x.Name`.
fn name_accessor() -> Accessor<Node, String> {
    Accessor::member("Name", |x: &Rc<Node>| x.name.clone())
}

/// `Flat()`: a flat source of "B" and "A" with two text columns of the name.
fn flat() -> Rc<FlatTreeDataGridSource<Node>> {
    let items: Rc<FerroList<Rc<Node>>> = Rc::new(FerroList::from_items([Node::new("B"), Node::new("A")]));
    let source = FlatTreeDataGridSource::new(items as Models<Node>);
    source.columns().add(core_models::TextColumn::new(header("One"), name_accessor(), None, None, None) as CoreColumnRef<Node>);
    source.columns().add(core_models::TextColumn::new(header("Two"), name_accessor(), None, None, None) as CoreColumnRef<Node>);
    source
}

/// `new TreeDataGridCellSelectionModel<Node>(source)`.
fn cell_selection(source: &Rc<FlatTreeDataGridSource<Node>>) -> Rc<TreeDataGridCellSelectionModel<Node>> {
    let typed: Rc<dyn ITreeDataGridSourceOf<Node>> = source.clone();
    TreeDataGridCellSelectionModel::new(&typed)
}

/// `source.Selection = selection`.
fn set_selection(source: &Rc<FlatTreeDataGridSource<Node>>, selection: &Rc<TreeDataGridCellSelectionModel<Node>>) {
    ITreeDataGridSource::set_selection(&**source, Some(selection.clone() as Rc<dyn ITreeDataGridSelection>));
}

fn counter() -> (Rc<Cell<i32>>, Rc<Cell<i32>>) {
    let count = Rc::new(Cell::new(0));
    (count.clone(), count)
}

fn cell_at(grid: &TreeDataGrid, column: i32, row: i32) -> Ref<TreeDataGridCell> {
    grid.try_get_row(row)
        .and_then(|row| row.try_get_cell(column))
        .and_then(|cell| cell.cast::<TreeDataGridCell>())
        .expect("a realized cell")
}

// ---------------------------------------------------------------------------------------------
// NativeColumn
// ---------------------------------------------------------------------------------------------

/// `NativeColumn`: a view column derived from `CellColumnBase<Node>` whose cells are layout test
/// cells of the name. The port's base is a field the column forwards the view contracts to.
struct NativeColumn {
    base: CellColumnBase,
}

impl NativeColumn {
    fn new(width: GridLength) -> Rc<Self> {
        Rc::new(Self { base: CellColumnBase::new(header("Name"), Some(width), CellColumnOptions::default()) })
    }
}

impl INotifyPropertyChanged for NativeColumn {
    fn property_changed(&self) -> &Event<str> {
        self.base.notifying().property_changed()
    }
}

impl IColumn for NativeColumn {
    fn actual_width(&self) -> f64 {
        self.base.actual_width()
    }

    fn can_user_resize(&self) -> Option<bool> {
        self.base.can_user_resize()
    }

    fn header(&self) -> Option<BoxedValue> {
        self.base.header()
    }

    fn width(&self) -> GridLength {
        self.base.width()
    }

    fn sort_direction(&self) -> Option<ListSortDirection> {
        self.base.sort_direction()
    }

    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.base.set_sort_direction(value);
    }

    fn tag(&self) -> Option<BoxedValue> {
        self.base.tag()
    }

    fn set_tag(&self, value: Option<BoxedValue>) {
        self.base.set_tag(value);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_update_column_layout(&self) -> Option<&dyn IUpdateColumnLayout> {
        Some(self)
    }

    fn as_cell_column(&self) -> Option<&dyn ICellColumn> {
        Some(self)
    }

    fn requires_unconstrained_width_measurement(&self) -> bool {
        self.base.requires_unconstrained_width_measurement()
    }
}

impl IUpdateColumnLayout for NativeColumn {
    fn min_actual_width(&self) -> f64 {
        self.base.min_actual_width()
    }

    fn max_actual_width(&self) -> f64 {
        self.base.max_actual_width()
    }

    fn star_width_was_constrained(&self) -> bool {
        self.base.star_width_was_constrained()
    }

    fn cell_measured(&self, width: f64, row_index: i32) -> f64 {
        self.base.cell_measured(width, row_index)
    }

    fn calculate_star_width(&self, available_width: f64, total_stars: f64) {
        self.base.calculate_star_width(available_width, total_stars);
    }

    fn commit_actual_width(&self) -> bool {
        self.base.commit_actual_width()
    }

    fn set_width(&self, width: GridLength) {
        self.base.set_width(width);
    }
}

impl ICellColumn for NativeColumn {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        let model = row.model().expect("a row with a model");
        let name = model.downcast_ref::<Node>().expect("a node").name.clone();
        LayoutTestCell::new(Some(name)) as CellRef
    }
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[test]
fn presentation_maps_sorted_rows_and_hidden_columns_without_legacy_sources() {
    let _app = start_app();
    let source = flat();
    let selection = cell_selection(&source);
    set_selection(&source, &selection);
    source.columns().get(0).set_is_visible(false);
    source.sort_by_column(&source.columns().get(1), ListSortDirection::Ascending);
    let presentation = TreeDataGridSourcePresentation::of::<Node, _>(source.clone(), None);

    // "A", the first visible cell.
    selection.set_selected_index(CellIndex::new(1, 1));
    let interaction = presentation.selection_interaction().expect("a selection interaction");
    assert!(interaction.is_cell_selected(0, 0));
    assert!(!interaction.is_cell_selected(0, 1));
    assert!(!interaction.is_cell_selected(-1, 0));
    assert!(!interaction.is_cell_selected(0, 20));
    // Cell selection does not enable row dragging.
    assert!(presentation.selected_indexes().is_none());
    assert!(!interaction.is_row_selected_at(0));

    ITreeDataGridSource::clear_sort(&*source);
    assert!(interaction.is_cell_selected(0, 1));
}

#[test]
fn selection_replacement_and_disposal_unsubscribe_old_interactions() {
    let _app = start_app();
    let source = flat();
    let first = cell_selection(&source);
    set_selection(&source, &first);
    let presentation = TreeDataGridSourcePresentation::of::<Node, _>(source.clone(), None);
    let (stale, count) = counter();
    presentation
        .selection_interaction()
        .expect("a selection interaction")
        .add_selection_changed(Rc::new(move || count.set(count.get() + 1)));

    first.set_selected_index(CellIndex::new(0, 0));
    assert_eq!(1, stale.get());

    let second = cell_selection(&source);
    set_selection(&source, &second);
    first.set_selected_index(CellIndex::new(1, 1));
    assert_eq!(1, stale.get());

    let (changes, count) = counter();
    presentation
        .selection_interaction()
        .expect("a selection interaction")
        .add_selection_changed(Rc::new(move || count.set(count.get() + 1)));
    second.set_selected_index(CellIndex::new(0, 1));
    assert_eq!(1, changes.get());

    presentation.dispose();
    second.clear();
    assert_eq!(1, changes.get());
}

#[test]
fn keyboard_selection_extends_backwards_and_restores_after_detach() {
    let _app = start_app();
    let source = flat();
    let selection = cell_selection(&source);
    selection.set_single_select(false);
    set_selection(&source, &selection);
    let grid = TreeDataGrid::new();
    grid.set_model(source_ref::<Node, _>(&source));
    grid.set_template(Some(tree_data_grid_template(0.0)));
    let window = test_window(Some(grid.clone().upcast()), None);
    window.styles().add(tree_data_grid_row_style());

    let press = |key: Key, modifiers: KeyModifiers| {
        let mut e = KeyEventArgs::new();
        e.set_routed_event(Some(InputElement::key_down_event()));
        e.key = key;
        e.key_modifiers = modifiers;
        grid.presentation()
            .expect("a presentation")
            .selection_interaction()
            .expect("a selection interaction")
            .on_key_down(&grid, &e);
        assert!(e.handled());
    };

    window.update_layout();
    selection.set_selected_index(CellIndex::new(1, 1));
    press(Key::Up, KeyModifiers::SHIFT);
    press(Key::Left, KeyModifiers::SHIFT);
    assert_eq!(4, selection.count());
    assert!(selection.is_selected(&CellIndex::new(0, 0)));
    press(Key::Down, KeyModifiers::NONE);
    assert_eq!(1, selection.selected_indexes().len());

    window.set_content(None);
    assert!(grid.presentation().expect("a presentation").selection_interaction().is_none());
    selection.set_selected_index(CellIndex::new(0, 0));
    window.set_content(Some(ferroui_controls::Control::boxed(&grid)));
    window.update_layout();
    assert!(grid
        .presentation()
        .expect("a presentation")
        .selection_interaction()
        .expect("a selection interaction")
        .is_cell_selected(0, 0));
    assert!(grid.source().is_none());

    window.close();
}

#[test]
fn pointer_selection_paints_cells_and_honors_cancellation_and_right_click() {
    let _app = start_app();
    let source = flat();
    let selection = cell_selection(&source);
    selection.set_single_select(false);
    set_selection(&source, &selection);
    let grid = TreeDataGrid::new();
    grid.set_model(source_ref::<Node, _>(&source));
    grid.set_template(Some(tree_data_grid_template(0.0)));
    let window = test_window(Some(grid.clone().upcast()), None);
    window.styles().add(tree_data_grid_row_style());

    window.update_layout();
    let interaction = grid
        .presentation()
        .expect("a presentation")
        .selection_interaction()
        .expect("a selection interaction");
    let cell = cell_at(&grid, 1, 1);
    let pointer: Rc<dyn IPointer> = Pointer::new(0, PointerType::Mouse, true);
    let cancel = Rc::new(Cell::new(true));
    let cancelling = cancel.clone();
    let _selection_changing = grid.selection_changing(move |e| e.set_cancel(cancelling.get()));
    let grid_visual: Ref<Visual> = grid.clone().upcast();
    let press = |right: bool| {
        let properties = if right {
            PointerPointProperties::new(RawInputModifiers::RIGHT_MOUSE_BUTTON, PointerUpdateKind::RightButtonPressed)
        } else {
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed)
        };
        let e = PointerPressedEventArgs::new(
            &cell,
            pointer.clone(),
            &grid_visual,
            Point::new(2.0, 2.0),
            0,
            properties,
            KeyModifiers::NONE,
            1,
        );
        interaction.on_pointer_pressed(&grid, &e);
    };

    press(false);
    assert!(selection.selected_indexes().is_empty());
    cancel.set(false);
    press(false);
    assert_eq!(CellIndex::new(1, 1), selection.selected_index());
    assert!(cell.is_selected());
    assert!(!cell_at(&grid, 0, 0).is_selected());

    selection.set_selected_range(&CellIndex::new(0, 0), 2, 2);
    press(true);
    assert_eq!(4, selection.count());
    let released = PointerReleasedEventArgs::new(
        &cell,
        pointer.clone(),
        &grid_visual,
        Point::new(2.0, 2.0),
        1,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::RightButtonReleased),
        KeyModifiers::NONE,
        MouseButton::Right,
    );
    interaction.on_pointer_released(&grid, &released);
    assert_eq!(4, selection.count());

    window.close();
}

fn native_columns_preserve_fixed_and_auto_measurement(auto: bool) {
    let _app = start_app();
    let source = flat();
    source.columns().clear();
    let width = if auto { CoreGridLength::AUTO } else { CoreGridLength::pixels(80.0) };
    let model = core_models::TextColumn::new(header("Name"), name_accessor(), Some(width), None, None);
    model.set_presentation_key(Some("custom".to_string()));
    source.columns().add(model as CoreColumnRef<Node>);
    let options = TreeDataGridPresentationOptions::new();
    options.set_column("custom", move |_: &Rc<dyn CoreColumn>| {
        NativeColumn::new(if auto { GridLength::AUTO } else { GridLength::from_pixels(80.0) }) as ColumnRef
    });
    let grid = TreeDataGrid::new();
    grid.set_presentation_options(Some(options as Rc<dyn ITreeDataGridPresentationOptions>));
    grid.set_model(source_ref::<Node, _>(&source));
    grid.set_template(Some(tree_data_grid_template(0.0)));
    grid.set_element_factory(test_element_factory());
    let window = test_window(Some(grid.clone().upcast()), None);
    window.styles().add(tree_data_grid_row_style());

    window.update_layout();
    let cell = grid
        .try_get_row(0)
        .and_then(|row| row.try_get_cell(0))
        .and_then(|cell| cell.cast::<LayoutTestCellControl>())
        .expect("a layout test cell control");
    let constraints = cell.measure_constraints();
    assert!(!constraints.is_empty());
    assert_eq!(if auto { f64::INFINITY } else { 80.0 }, constraints[0].width);
    let columns = grid.columns().expect("columns");
    assert_eq!(auto, columns.get(0).requires_unconstrained_width_measurement());
    // `Assert.False(grid.Columns[0] is IColumn<Node>)`: the native column is a view column only,
    // not a column of the classic typed API, which is also a core column in the port.
    assert!(columns.get(0).core_column().is_none());
    // `Assert.True(typeof(IColumnMeasurementOptions).IsPublic)`: the measurement option is a
    // member of the public view contract (`IColumn::requires_unconstrained_width_measurement`),
    // which the call above reaches from outside the presentation.

    window.close();
}

#[test]
fn native_columns_preserve_fixed_and_auto_measurement_fixed() {
    native_columns_preserve_fixed_and_auto_measurement(false);
}

#[test]
fn native_columns_preserve_fixed_and_auto_measurement_auto() {
    native_columns_preserve_fixed_and_auto_measurement(true);
}

#[test]
fn v12_selection_mode_creates_native_cells_and_reports_cell_deltas() {
    let _app = start_app();
    let source = flat();
    let grid = TreeDataGrid::new();
    grid.set_selection_mode(TreeDataGridSelectionMode::CELL | TreeDataGridSelectionMode::MULTIPLE);
    grid.set_model(source_ref::<Node, _>(&source));
    let selection = downcast_selection::<TreeDataGridCellSelectionModel<Node>>(
        &ITreeDataGridSource::selection(&*source).expect("a selection"),
    )
    .expect("a cell selection");
    assert!(!selection.single_select());
    let window = test_window(Some(grid.clone().upcast()), None);

    // The last change: its deselected and selected cells.
    let changed: Rc<RefCell<Option<(Vec<CellIndex>, Vec<CellIndex>)>>> = Rc::new(RefCell::new(None));
    let recorder = changed.clone();
    let _selection_changed = grid.selection_changed(move |e| {
        *recorder.borrow_mut() = Some((e.deselected_cell_indexes().to_vec(), e.selected_cell_indexes().to_vec()));
    });

    selection.set_selected_index(CellIndex::new(1, 0));
    let (_, selected) = changed.borrow().clone().expect("a selection change");
    assert_eq!(vec![CellIndex::new(1, 0)], selected);

    selection.set_selected_index(CellIndex::new(0, 1));
    let (deselected, selected) = changed.borrow().clone().expect("a selection change");
    assert_eq!(vec![CellIndex::new(1, 0)], deselected);
    assert_eq!(vec![CellIndex::new(0, 1)], selected);

    grid.set_selection_mode(TreeDataGridSelectionMode::ROW);
    assert!(downcast_selection::<TreeDataGridRowSelectionModel<Node>>(
        &ITreeDataGridSource::selection(&*source).expect("a selection")
    )
    .is_some());
    *changed.borrow_mut() = None;
    selection.clear();
    assert!(changed.borrow().is_none());

    grid.set_selection_mode(TreeDataGridSelectionMode::CELL);
    let cells = downcast_selection::<TreeDataGridCellSelectionModel<Node>>(
        &ITreeDataGridSource::selection(&*source).expect("a selection"),
    )
    .expect("a cell selection");
    assert!(cells.single_select());

    window.close();
    selection.dispose();
}
