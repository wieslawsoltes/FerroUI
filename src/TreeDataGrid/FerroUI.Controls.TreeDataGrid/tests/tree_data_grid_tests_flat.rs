//! `TreeDataGridTests_Flat.cs`: the grid over a flat source: the rows and cells it realizes, the
//! subscriptions of its cells to the models, the reuse of rows and cells while scrolling and while
//! the items change, the sizing of its columns, its cell events, its selection while sorting and
//! removing items, and its scroll bars.
//!
//! The reference's classic `FlatTreeDataGridSource<Model>` with classic columns is the core's
//! source holding the view columns of `crate::models` (which are core columns too); the source is
//! shown through `Source`. `target.Columns` are the view columns of the grid's presentation, which
//! are the same column objects, and `target.Source.SortBy(target.Columns[0], ..)` sorts the source
//! by the core column of that view column.

use crate::tests::support::*;

use crate::models::{
    CellRef, CheckBoxCell, CheckBoxColumn, ColumnRef, ICell, IColumn, IColumns, ITextCell, ITreeDataGridRows,
    IViewColumn, TemplateCell, TemplateColumn, TextColumn, TextColumnOptions,
};
use crate::primitives::{
    TreeDataGridCell, TreeDataGridCheckBoxCell, TreeDataGridColumnHeader, TreeDataGridRow, TreeDataGridTemplateCell,
    TreeDataGridTextCell,
};
use crate::selection::ITreeDataGridSelectionInteraction;
use crate::{TreeDataGrid, TreeDataGridCellEventArgs, TreeDataGridRowDragStartedEventArgs};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{Maybe, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model as ModelType};
use ferroui_base::input::{
    DragDropEffects, InputElement, KeyModifiers, Pointer, PointerEventArgs, PointerPointProperties,
    PointerPressedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::layout::Layoutable;
use ferroui_base::media::{Colors, IBrush, SolidColorBrush};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_model, ferro_properties, ferro_static_type, AttachedProperty, BoxedValue, FerroProperty, Point, Ref, Size,
    StyledElement, Vector, Visual,
};
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::{Border, Control, GridLength, GridUnitType, ScrollViewer, TopLevel, Window};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use ferroui_treedatagrid_core::model_collection::Models;
use ferroui_treedatagrid_core::models::{ColumnRef as CoreColumnRef, ListSortDirection};
use ferroui_treedatagrid_core::selection::ITreeSelectionModel;
use ferroui_treedatagrid_core::{Accessor, FlatTreeDataGridSource, ITreeDataGridSource, IndexPath};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// The resource property (`TestResourceProperty` of `ResourcePropertyOwner`)
// ---------------------------------------------------------------------------------------------

struct ResourcePropertyOwner;

ferro_static_type!(ResourcePropertyOwner);

ferro_properties! {
    impl ResourcePropertyOwner {
        fn test_resource_property() -> AttachedProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register_attached::<ResourcePropertyOwner, StyledElement, _>("TestResource", None)
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------------------------

/// `Model` of the tests: a `NotifyingBase` with an id, an active flag and a title.
struct Model {
    id: Cell<i32>,
    is_active: Cell<bool>,
    title: RefCell<Option<String>>,
    notifier: TestNotifier,
}

ferro_model!(Model, |b| b
    .notify_property_changed()
    .property::<Value<i32>>("Id", |m| m.id(), |m, v| m.set_id(v))
    .property::<Value<bool>>("IsActive", |m| m.is_active(), |m, v| m.set_is_active(v))
    .property::<Maybe<String>>("Title", |m| m.title(), |m, v| m.set_title(v)));

impl INotifyPropertyChanged for Model {
    fn property_changed(&self) -> &Event<str> {
        self.notifier.event()
    }
}

impl Model {
    fn new(id: i32, title: &str) -> Rc<Self> {
        <Model as ModelType>::new_model(Self {
            id: Cell::new(id),
            is_active: Cell::new(false),
            title: RefCell::new(Some(title.to_string())),
            notifier: TestNotifier::new(),
        })
    }

    fn id(&self) -> i32 {
        self.id.get()
    }

    // `Id { get; set; }` does not notify.
    fn set_id(&self, value: i32) {
        self.id.set(value);
    }

    fn is_active(&self) -> bool {
        self.is_active.get()
    }

    fn set_is_active(&self, value: bool) {
        self.notifier.raise_and_set_cell_if_changed(&self.is_active, value, "IsActive");
    }

    fn title(&self) -> Option<String> {
        self.title.borrow().clone()
    }

    fn set_title(&self, value: Option<String>) {
        self.notifier.raise_and_set_if_changed(&self.title, value, "Title");
    }

    fn property_changed_subscriber_count(&self) -> usize {
        self.notifier.property_changed_subscriber_count()
    }

    /// `x => x.Id`.
    fn id_accessor() -> Accessor<Model, i32> {
        Accessor::member("Id", |m: &Rc<Model>| m.id())
    }

    /// `x => x.Title`.
    fn title_accessor() -> Accessor<Model, Option<String>> {
        Accessor::member("Title", |m: &Rc<Model>| m.title())
    }

    /// `x => x.Title, (o, v) => o.Title = v`.
    fn title_accessor_with_setter() -> Accessor<Model, Option<String>> {
        Model::title_accessor().with_setter(|m: &Rc<Model>, v: Option<String>| m.set_title(v))
    }

    /// `x => x.IsActive, (x, v) => x.IsActive = v`.
    fn is_active_accessor_with_setter() -> Accessor<Model, bool> {
        Accessor::member("IsActive", |m: &Rc<Model>| m.is_active())
            .with_setter(|m: &Rc<Model>, v: bool| m.set_is_active(v))
    }
}

// ---------------------------------------------------------------------------------------------
// Columns
// ---------------------------------------------------------------------------------------------

/// A column of the tests: the view column (whose `ActualWidth` the tests read) and the same column
/// as a core column of the source.
#[derive(Clone)]
struct Col {
    view: ColumnRef,
    core: CoreColumnRef<Model>,
}

fn col<C: IViewColumn<Model> + 'static>(column: Rc<C>) -> Col {
    Col { view: column.clone(), core: column }
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

fn pixels(value: f64) -> GridLength {
    GridLength::new(value, GridUnitType::Pixel)
}

fn stars(value: f64) -> GridLength {
    GridLength::new(value, GridUnitType::Star)
}

/// `new TextColumn<Model, int>(header, x => x.Id, width, options)`.
fn id_column(name: &str, width: Option<GridLength>, options: Option<Rc<TextColumnOptions<Model>>>) -> Col {
    col(TextColumn::<Model, i32>::new(header(name), Model::id_accessor(), width, options))
}

/// `new TextColumn<Model, string?>(header, x => x.Title, width, options)`.
fn title_column(name: &str, width: Option<GridLength>, options: Option<Rc<TextColumnOptions<Model>>>) -> Col {
    col(TextColumn::<Model, Option<String>>::new(header(name), Model::title_accessor(), width, options))
}

/// `MinWidth(min)`.
fn min_width(min: f64) -> Option<Rc<TextColumnOptions<Model>>> {
    let options = TextColumnOptions::new();
    options.set_min_width(pixels(min));
    Some(Rc::new(options))
}

/// `MaxWidth(max)`.
fn max_width(max: f64) -> Option<Rc<TextColumnOptions<Model>>> {
    let options = TextColumnOptions::new();
    options.set_max_width(Some(pixels(max)));
    Some(Rc::new(options))
}

/// `new TextColumnOptions<Model> { StringFormat = format }`.
fn string_format(format: &str) -> Option<Rc<TextColumnOptions<Model>>> {
    let options = TextColumnOptions::new();
    options.set_string_format(format);
    Some(Rc::new(options))
}

/// `new FlatTreeDataGridSource<Model>(items) { Columns = { ... } }`.
fn new_source(items: &Rc<FerroList<Rc<Model>>>, columns: &[Col]) -> Rc<FlatTreeDataGridSource<Model>> {
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Model>);
    for column in columns {
        source.columns().add(column.core.clone());
    }
    source
}

// ---------------------------------------------------------------------------------------------
// Target
// ---------------------------------------------------------------------------------------------

struct Target {
    target: Ref<TreeDataGrid>,
    items: Rc<FerroList<Rc<Model>>>,
    source: Rc<FlatTreeDataGridSource<Model>>,
    root: Ref<Window>,
}

impl Target {
    fn scroll(&self) -> Ref<ScrollViewer> {
        self.target.scroll().expect("a scroll viewer")
    }

    fn row_selection(&self) -> Rc<dyn ITreeSelectionModel> {
        self.target.row_selection().expect("a row selection")
    }

    fn item(&self, index: usize) -> Rc<Model> {
        self.items.get(index)
    }
}

struct Options {
    models: Option<Vec<Rc<Model>>>,
    columns: Option<Vec<Col>>,
    item_count: usize,
    run_layout: bool,
    header_presenter_size: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self { models: None, columns: None, item_count: 100, run_layout: true, header_presenter_size: 0.0 }
    }
}

/// `CreateTarget(models, columns, itemCount, runLayout, headerPresenterSize)`.
fn create_target_with(options: Options) -> Target {
    let items = match options.models {
        None => Rc::new(FerroList::from_items((0..options.item_count).map(|x| Model::new(x as i32, &format!("Item {x}"))))),
        Some(models) => Rc::new(FerroList::from_items(models)),
    };

    let source = FlatTreeDataGridSource::new(items.clone() as Models<Model>);
    source.row_selection().expect("a row selection").base().set_single_select(false);

    match options.columns {
        Some(columns) => {
            for column in columns {
                source.columns().add(column.core);
            }
        }
        None => {
            source.columns().add(id_column("ID", None, None).core);
            source.columns().add(
                col(TextColumn::<Model, Option<String>>::new(
                    header("Title"),
                    Model::title_accessor_with_setter(),
                    None,
                    None,
                ))
                .core,
            );
        }
    }

    let target = TreeDataGrid::new();
    target.set_template(Some(tree_data_grid_template(options.header_presenter_size)));
    target.set_source(source_ref::<Model, _>(&source));

    let root = test_window(Some(target.clone().upcast::<Control>()), None);
    root.styles().add(tree_data_grid_row_style());
    root.styles().add(Style::with_setters(
        Selectors::of_type::<TreeDataGridCell>(),
        [Setter::new(Layoutable::height_property(), 10.0)],
    ));

    if options.run_layout {
        root.update_layout();
        run_jobs();
    }

    Target { target, items, source, root }
}

fn create_target() -> Target {
    create_target_with(Options::default())
}

fn create_target_with_columns(columns: Vec<Col>) -> Target {
    create_target_with(Options { columns: Some(columns), ..Options::default() })
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn range(first: i32, count: i32) -> Vec<i32> {
    (first..first + count).collect()
}

/// The visible rows of the rows presenter (`RowsPresenter.GetVisualChildren().Cast<TreeDataGridRow>()`).
fn visual_rows(target: &TreeDataGrid) -> Vec<Ref<TreeDataGridRow>> {
    let presenter = target.rows_presenter().expect("a rows presenter");
    visual_children_of::<TreeDataGridRow>(&presenter)
}

/// The cells among the visual children of the cells presenter of `row`.
fn visual_cells(row: &TreeDataGridRow) -> Vec<Ref<TreeDataGridCell>> {
    let presenter = row.cells_presenter().expect("a cells presenter");
    visual_children_of::<TreeDataGridCell>(&presenter)
}

/// `row.CellsPresenter.RealizedElements.Cast<TreeDataGridCell>()`.
fn realized_cells(row: &TreeDataGridRow) -> Vec<Ref<TreeDataGridCell>> {
    row.cells_presenter()
        .expect("a cells presenter")
        .realized_elements()
        .into_iter()
        .flatten()
        .map(|element| element.cast::<TreeDataGridCell>().expect("a cell"))
        .collect()
}

/// `row.CellsPresenter.GetRealizedElements().Cast<TreeDataGridCell>().OrderBy(x => x.ColumnIndex)`.
fn realized_cells_by_column(row: &TreeDataGridRow) -> Vec<Ref<TreeDataGridCell>> {
    let mut cells: Vec<Ref<TreeDataGridCell>> = row
        .cells_presenter()
        .expect("a cells presenter")
        .get_realized_elements()
        .into_iter()
        .map(|element| element.cast::<TreeDataGridCell>().expect("a cell"))
        .collect();
    cells.sort_by_key(|cell| cell.column_index());
    cells
}

/// `target.RowsPresenter.RealizedElements.Cast<TreeDataGridRow>()`.
fn realized_grid_rows(target: &TreeDataGrid) -> Vec<Ref<TreeDataGridRow>> {
    target
        .rows_presenter()
        .expect("a rows presenter")
        .realized_elements()
        .into_iter()
        .flatten()
        .map(|element| element.cast::<TreeDataGridRow>().expect("a row"))
        .collect()
}

fn row_at(target: &TreeDataGrid, index: i32) -> Ref<TreeDataGridRow> {
    target
        .rows_presenter()
        .expect("a rows presenter")
        .try_get_element(index)
        .and_then(|element| element.cast::<TreeDataGridRow>())
        .expect("a row")
}

fn same_cell(a: Option<&CellRef>, b: Option<&CellRef>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
        (None, None) => true,
        _ => false,
    }
}

fn same_brush(brush: &Rc<dyn IBrush>, value: Option<Rc<dyn IBrush>>) -> bool {
    value.is_some_and(|value| std::ptr::addr_eq(Rc::as_ptr(&value), Rc::as_ptr(brush)))
}

fn check_box_cell(cell: &CellRef) -> &CheckBoxCell {
    cell.as_any().downcast_ref::<CheckBoxCell>().expect("a check box cell")
}

fn template_cell(cell: &CellRef) -> &TemplateCell {
    cell.as_any().downcast_ref::<TemplateCell>().expect("a template cell")
}

/// `target.Source.SortBy(target.Columns[0], direction)`.
fn sort_by_first_column(target: &TreeDataGrid, direction: ListSortDirection) {
    let column = target.columns().expect("columns").get(0);
    let core = column.core_column().expect("a view column of a core column");
    let source = target.source().expect("a source");
    source.sort_by(Some(&*core), direction);
}

/// `AssertRowIndexes(target, firstRowIndex, rowCount)`.
fn assert_row_indexes(target: &TreeDataGrid, first_row_index: i32, row_count: i32) {
    assert!(target.rows_presenter().is_some());

    let mut row_indexes: Vec<i32> =
        visual_rows(target).iter().filter(|x| x.is_visible()).map(|x| x.row_index()).collect();
    row_indexes.sort();

    assert_eq!(range(first_row_index, row_count), row_indexes);

    let mut row_indexes: Vec<i32> =
        realized_grid_rows(target).iter().filter(|x| x.is_visible()).map(|x| x.row_index()).collect();
    row_indexes.sort();

    assert_eq!(range(first_row_index, row_count), row_indexes);
}

/// `AssertColumnIndexes(target, firstColumnIndex, columnCount)`.
fn assert_column_indexes(target: &TreeDataGrid, first_column_index: i32, column_count: i32) {
    let presenter = target.column_headers_presenter().expect("a column headers presenter");

    let mut column_indexes: Vec<i32> = visual_children_of::<TreeDataGridColumnHeader>(&presenter)
        .iter()
        .filter(|x| x.is_visible())
        .map(|x| x.column_index())
        .collect();
    column_indexes.sort();

    assert_eq!(range(first_column_index, column_count), column_indexes);

    let mut column_indexes: Vec<i32> = presenter
        .realized_elements()
        .into_iter()
        .flatten()
        .map(|element| element.cast::<TreeDataGridColumnHeader>().expect("a column header"))
        .filter(|x| x.is_visible())
        .map(|x| x.column_index())
        .collect();
    column_indexes.sort();

    assert_eq!(range(first_column_index, column_count), column_indexes);
}

/// `AssertInteractionSelection(target, selected)`.
fn assert_interaction_selection(target: &TreeDataGrid, selected: &[i32]) {
    let selection = target.selection_interaction().expect("a selection interaction");
    let count = target.rows().expect("rows").count();

    for i in 0..count as i32 {
        assert_eq!(selected.contains(&i), selection.is_row_selected_at(i), "row {i}");
    }
}

/// `AssertModelSubscriptions(items)`.
fn assert_model_subscriptions(items: &FerroList<Rc<Model>>) {
    for i in 0..items.count() {
        let expected = if i < 10 { 2 } else { 0 };
        assert_eq!(expected, items.get(i).property_changed_subscriber_count(), "item {i}");
    }
}

fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

fn increment(counter: &Cell<i32>) {
    counter.set(counter.get() + 1);
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[test]
fn should_display_initial_rows_and_cells() {
    let _app = start_app();
    let t = create_target();

    assert!(t.target.rows_presenter().is_some());

    let rows = visual_rows(&t.target);

    assert_eq!(10, rows.len());

    for row in &rows {
        let cells = visual_cells(row);
        assert_eq!(2, cells.len());
    }
}

#[test]
fn dynamic_resources_propagate_through_virtualized_control_trees() {
    let _app = start_app();
    const RESOURCE_KEY: &str = "TreeDataGridTestBrush";
    let first_brush: Rc<dyn IBrush> = SolidColorBrush::with_color(Colors::RED).into();
    let second_brush: Rc<dyn IBrush> = SolidColorBrush::with_color(Colors::BLUE).into();
    let t = create_target();
    let target = &t.target;
    let headers_presenter = target.column_headers_presenter().expect("a column headers presenter");
    let rows_presenter = target.rows_presenter().expect("a rows presenter");
    let header = headers_presenter.realized_elements()[0]
        .as_ref()
        .and_then(|element| element.cast::<TreeDataGridColumnHeader>())
        .expect("a column header");
    let row = rows_presenter.try_get_element(0).and_then(|element| element.cast::<TreeDataGridRow>()).expect("a row");
    let cells_presenter = row.cells_presenter().expect("a cells presenter");
    let cell = cells_presenter.realized_elements()[0]
        .as_ref()
        .and_then(|element| element.cast::<TreeDataGridCell>())
        .expect("a cell");
    let elements: Vec<Ref<StyledElement>> = vec![
        target.clone().upcast::<StyledElement>(),
        headers_presenter.clone().upcast::<StyledElement>(),
        header.clone().upcast::<StyledElement>(),
        rows_presenter.clone().upcast::<StyledElement>(),
        row.clone().upcast::<StyledElement>(),
        cells_presenter.clone().upcast::<StyledElement>(),
        cell.clone().upcast::<StyledElement>(),
    ];

    target.resources().set(RESOURCE_KEY, Some(Rc::new(first_brush.clone()) as BoxedValue));

    let property = ResourcePropertyOwner::test_resource_property();
    let mut expressions = Vec::new();
    for element in &elements {
        let extension = DynamicResourceExtension::with_resource_key(Some(Rc::new(RESOURCE_KEY.to_string()) as BoxedValue));
        expressions.push(element.bind_binding(property.as_property(), &*extension));
    }

    for element in &elements {
        assert!(same_brush(&first_brush, element.get_value(property)), "{element:?}");
    }

    t.scroll().set_offset(Vector::new(0.0, 10.0));
    layout(target);

    assert!(rows_presenter.try_get_element(10).is_some_and(|element| element.ptr_eq(&row)));
    assert!(row.cells_presenter().expect("a cells presenter").realized_elements()[0]
        .as_ref()
        .is_some_and(|element| element.ptr_eq(&cell)));
    for element in &elements {
        assert!(same_brush(&first_brush, element.get_value(property)), "{element:?}");
    }

    target.resources().set(RESOURCE_KEY, Some(Rc::new(second_brush.clone()) as BoxedValue));
    run_jobs();

    for element in &elements {
        assert!(same_brush(&second_brush, element.get_value(property)), "{element:?}");
    }
}

#[test]
fn multi_selection_should_work_correctly_with_duplicates() {
    let _app = start_app();
    let mut items = vec![Model::new(0, "Item 0"), Model::new(1, "Item 1"), Model::new(2, "Item 2")];
    items.push(items[0].clone());
    items.push(items[0].clone());
    items.push(items[0].clone());

    let t = create_target_with(Options { models: Some(items), ..Options::default() });

    t.row_selection().select(&IndexPath::new(3));
    t.row_selection().select(&IndexPath::new(4));
    t.row_selection().select(&IndexPath::new(5));

    assert_interaction_selection(&t.target, &[3, 4, 5]);

    sort_by_first_column(&t.target, ListSortDirection::Ascending);

    assert_interaction_selection(&t.target, &[1, 2, 3]);
}

#[test]
fn selection_should_be_preserved_after_sorting() {
    let _app = start_app();
    let t = create_target();

    t.row_selection().select(&IndexPath::new(0));
    t.row_selection().select(&IndexPath::new(5));

    assert_interaction_selection(&t.target, &[0, 5]);

    sort_by_first_column(&t.target, ListSortDirection::Descending);

    // There are 100 items in the collection. Their IDs are in range 0..99 so when we order the IDs
    // column in descending order the last element of the collection is the one with ID 0 (index 99
    // in the collection), the first the one with ID 99.
    assert_interaction_selection(&t.target, &[94, 99]);
}

#[test]
fn collection_edit_preserves_selected_row_control_with_its_item() {
    let _app = start_app();
    let t = create_target();
    let selected_item = t.item(5);
    let selected_row = row_at(&t.target, 5);

    t.row_selection().select(&IndexPath::new(5));
    assert!(selected_row.is_selected());

    t.items.insert(2, Model::new(-1, "Inserted"));
    layout(&t.target);

    assert!(Rc::ptr_eq(&selected_item, &t.item(6)));
    assert_eq!(IndexPath::new(6), t.row_selection().selected_index());
    assert!(row_at(&t.target, 6).ptr_eq(&selected_row));
    assert!(selected_row.is_selected());
    assert_model_subscriptions(&t.items);

    t.items.remove_at(2);
    layout(&t.target);

    assert!(Rc::ptr_eq(&selected_item, &t.item(5)));
    assert_eq!(IndexPath::new(5), t.row_selection().selected_index());
    assert!(row_at(&t.target, 5).ptr_eq(&selected_row));
    assert!(selected_row.is_selected());
    assert_model_subscriptions(&t.items);
}

#[test]
fn collection_edit_preserves_row_cell_controls_and_models_with_their_item() {
    let _app = start_app();
    let t = create_target();
    let retained_item = t.item(5);
    let retained_row = row_at(&t.target, 5);
    let retained_cells = realized_cells(&retained_row);
    let retained_cell_models: Vec<Option<CellRef>> = retained_cells.iter().map(|x| x.model()).collect();

    assert_eq!(2, retained_cells.len());

    let assert_retained_row_and_cells = |row_index: i32| {
        let row = row_at(&t.target, row_index);
        assert!(Rc::ptr_eq(&retained_item, &t.item(row_index as usize)));
        assert!(row.ptr_eq(&retained_row));
        assert!(same_model(&retained_item, row.data_context().as_ref()));
        assert_eq!(retained_cells, realized_cells(&row));

        for (cell, model) in retained_cells.iter().zip(&retained_cell_models) {
            assert_eq!(row_index, cell.row_index());
            assert!(same_cell(model.as_ref(), cell.model().as_ref()));
        }
    };

    t.items.insert(2, Model::new(-1, "Inserted"));
    layout(&t.target);

    assert_retained_row_and_cells(6);

    t.items.remove_at(2);
    layout(&t.target);

    assert_retained_row_and_cells(5);
}

#[test]
fn should_subscribe_to_models_for_initial_rows() {
    let _app = start_app();
    let t = create_target();

    for i in 0..t.items.count() {
        let expected = if i < 10 { 2 } else { 0 };
        assert_eq!(expected, t.item(i).property_changed_subscriber_count(), "item {i}");
    }
}

#[test]
fn should_subscribe_to_correct_models_after_scrolling_down_one_row() {
    let _app = start_app();
    let t = create_target();

    t.scroll().set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    for i in 0..t.items.count() {
        let expected = if i > 0 && i <= 10 { 2 } else { 0 };
        assert_eq!(expected, t.item(i).property_changed_subscriber_count(), "item {i}");
    }
}

#[test]
fn full_row_scroll_reuses_row_cell_and_binding_instances() {
    let _app = start_app();
    let t = create_target();
    let row = row_at(&t.target, 0);
    let cells = realized_cells(&row);
    let models: Vec<Option<CellRef>> = cells.iter().map(|x| x.model()).collect();

    t.scroll().set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    let rebound = row_at(&t.target, 10);
    assert!(rebound.ptr_eq(&row));
    assert!(same_model(&t.item(10), rebound.data_context().as_ref()));
    assert_eq!(cells, realized_cells(&rebound));
    assert!(cells.iter().all(|cell| cell.row_index() == 10));
    for (cell, model) in cells.iter().zip(&models) {
        assert!(same_cell(model.as_ref(), cell.model().as_ref()));
    }

    let title_model = cells[1].model().expect("a cell model");
    let title_cell = title_model.as_text_cell().expect("a text cell");
    t.item(0).set_title(Some("Old row changed".to_string()));
    assert_eq!(Some("Item 10".to_string()), title_cell.text());
    t.item(10).set_title(Some("Rebound row changed".to_string()));
    assert_eq!(Some("Rebound row changed".to_string()), title_cell.text());
    title_cell.set_text(Some("Written through rebound cell".to_string()));
    assert_eq!(Some("Written through rebound cell".to_string()), t.item(10).title());

    for i in 0..t.items.count() {
        let expected = if i > 0 && i <= 10 { 2 } else { 0 };
        assert_eq!(expected, t.item(i).property_changed_subscriber_count(), "item {i}");
    }
}

#[test]
fn full_row_scroll_reuses_check_box_and_template_cell_models() {
    let _app = start_app();
    let template: Rc<dyn IDataTemplate> = FuncDataTemplate::new(
        |data| data.is_some_and(|data| data.downcast_ref::<Model>().is_some()),
        |_, _| Some(Border::new().upcast::<Control>()),
        true,
    );
    let columns = vec![
        col(CheckBoxColumn::<Model>::new(header("Active"), Model::is_active_accessor_with_setter(), None, None)),
        col(TemplateColumn::<Model>::new(header("Template"), template, None, None, None)),
    ];
    let t = create_target_with_columns(columns);
    let row = row_at(&t.target, 0);
    let cells = realized_cells(&row);
    let check_box_model = cells[0].model().expect("a cell model");
    let template_model = cells[1].model().expect("a cell model");
    check_box_cell(&check_box_model);
    template_cell(&template_model);

    t.scroll().set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    let rebound = row_at(&t.target, 10);
    assert!(rebound.ptr_eq(&row));
    let rebound_elements = rebound.cells_presenter().expect("a cells presenter").realized_elements();
    let rebound_check_box = rebound_elements[0]
        .as_ref()
        .and_then(|element| element.cast::<TreeDataGridCheckBoxCell>())
        .expect("a check box cell");
    assert!(same_cell(Some(&check_box_model), rebound_check_box.model().as_ref()));
    let rebound_template = rebound_elements[1]
        .as_ref()
        .and_then(|element| element.cast::<TreeDataGridTemplateCell>())
        .expect("a template cell");
    assert!(same_cell(Some(&template_model), rebound_template.model().as_ref()));
    assert!(same_model(&t.item(10), template_cell(&template_model).value().as_ref()));
    assert!(same_model(&t.item(10), rebound_template.content().as_ref()));

    t.item(0).set_is_active(true);
    assert_eq!(Some(false), check_box_cell(&check_box_model).value());
    t.item(10).set_is_active(true);
    assert_eq!(Some(true), check_box_cell(&check_box_model).value());
    check_box_cell(&check_box_model).set_value(Some(false));
    assert!(!t.item(10).is_active());
    assert!(t.item(0).is_active());
}

#[test]
fn detach_reattach_preserves_cell_control_subtrees() {
    let _app = start_app();
    let t = create_target();
    let window = t.root.clone();
    assert_eq!(Some(window.clone().upcast::<StyledElement>()), t.target.parent());
    let cells: Vec<Ref<TreeDataGridCell>> =
        realized_grid_rows(&t.target).iter().flat_map(|row| realized_cells(row)).collect();
    let models: Vec<Option<CellRef>> = cells.iter().map(|x| x.model()).collect();
    let logical_parents: Vec<Option<Ref<StyledElement>>> = cells.iter().map(|x| x.parent()).collect();
    let visual_parents: Vec<Option<Ref<Visual>>> = cells.iter().map(|x| x.get_visual_parent()).collect();
    let clearing_raised = counter();
    let prepared_raised = counter();

    let clearing = clearing_raised.clone();
    let _clearing = t.target.cell_clearing(move |_| increment(&clearing));
    let prepared = prepared_raised.clone();
    let _prepared = t.target.cell_prepared(move |_| increment(&prepared));

    window.set_content(None);
    window.update_layout();

    for (cell, parent) in cells.iter().zip(&logical_parents) {
        assert_eq!(*parent, cell.parent());
    }
    for (cell, parent) in cells.iter().zip(&visual_parents) {
        assert_eq!(*parent, cell.get_visual_parent());
    }

    window.set_content(Some(Control::boxed(t.target.clone().upcast::<Control>())));
    window.update_layout();
    run_jobs();

    let rebound_cells: Vec<Ref<TreeDataGridCell>> =
        realized_grid_rows(&t.target).iter().flat_map(|row| realized_cells(row)).collect();

    assert_eq!(cells, rebound_cells);
    assert_eq!(cells.len() as i32, clearing_raised.get());
    assert_eq!(cells.len() as i32, prepared_raised.get());
    for (model, cell) in models.iter().zip(&rebound_cells) {
        assert!(!same_cell(model.as_ref(), cell.model().as_ref()));
    }
    assert_model_subscriptions(&t.items);
}

#[test]
fn should_subscribe_to_correct_models_after_scrolling_down_one_page() {
    let _app = start_app();
    let t = create_target();

    t.scroll().set_offset(Vector::new(0.0, 100.0));
    layout(&t.target);

    for i in 0..t.items.count() {
        let expected = if (10..20).contains(&i) { 2 } else { 0 };
        assert_eq!(expected, t.item(i).property_changed_subscriber_count(), "item {i}");
    }
}

#[test]
fn should_unsubscribe_from_models_when_detached_from_logical_tree() {
    let _app = start_app();
    let t = create_target();

    t.root.set_content(None);

    for i in 0..t.items.count() {
        assert_eq!(0, t.item(i).property_changed_subscriber_count(), "item {i}");
    }
}

#[test]
fn auto_drag_drop_rows_should_raise_row_drag_started_when_row_is_dragged() {
    let _app = start_app();
    let t = create_target();
    let row = visual_rows(&t.target).into_iter().next().expect("a row");
    let pointer = Pointer::new(0, PointerType::Mouse, true);
    let initial_effects: Rc<Cell<Option<DragDropEffects>>> = Rc::new(Cell::new(None));

    t.target.set_auto_drag_drop_rows(true);
    t.row_selection().select(&IndexPath::new(0));
    let effects = initial_effects.clone();
    let _token = t.target.row_drag_started(move |_, e: &TreeDataGridRowDragStartedEventArgs| {
        effects.set(Some(e.allowed_effects()));
        e.set_allowed_effects(DragDropEffects::NONE);
    });

    let row_visual: &Visual = &row;
    row.raise_event(&PointerPressedEventArgs::new(
        &row,
        pointer.clone(),
        row_visual,
        Point::new(1.0, 1.0),
        0,
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    ));

    row.raise_event(&PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()),
        &row,
        pointer.clone(),
        Some(row_visual),
        Point::new(10.0, 10.0),
        1,
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::Other),
        KeyModifiers::NONE,
    ));

    assert_eq!(Some(DragDropEffects::MOVE), initial_effects.get());
}

#[test]
fn desired_width_should_be_total_of_fixed_width_columns() {
    let _app = start_app();
    let t = create_target_with_columns(vec![
        id_column("ID", Some(pixels(10.0)), min_width(0.0)),
        title_column("Title", Some(pixels(14.0)), min_width(0.0)),
    ]);

    assert_eq!(24.0, t.target.desired_size().width);
}

#[test]
fn should_size_star_columns() {
    let _app = start_app();
    let t = create_target_with_columns(vec![
        id_column("ID", Some(stars(1.0)), min_width(0.0)),
        title_column("Title", Some(stars(3.0)), min_width(0.0)),
    ]);

    let rows = visual_rows(&t.target);

    assert_eq!(10, rows.len());

    for row in &rows {
        let cells = visual_cells(row);
        assert_eq!(2, cells.len());
        assert_eq!(25.0, cells[0].bounds().width);
        assert_eq!(75.0, cells[1].bounds().width);
    }
}

#[test]
fn should_size_star_columns_with_min_width() {
    let _app = start_app();
    let t = create_target_with_columns(vec![
        id_column("ID", Some(stars(1.0)), min_width(50.0)),
        title_column("Title", Some(stars(3.0)), None),
    ]);

    let rows = visual_rows(&t.target);

    assert_eq!(10, rows.len());

    for row in &rows {
        let cells = visual_cells(row);
        assert_eq!(2, cells.len());
        assert_eq!(50.0, cells[0].bounds().width);
        assert_eq!(50.0, cells[1].bounds().width);
    }
}

#[test]
fn should_size_star_columns_with_max_width() {
    let _app = start_app();
    let t = create_target_with_columns(vec![
        id_column("ID", Some(stars(1.0)), None),
        title_column("Title", Some(stars(1.0)), max_width(25.0)),
    ]);

    let rows = visual_rows(&t.target);

    assert_eq!(10, rows.len());

    for row in &rows {
        let cells = visual_cells(row);
        assert_eq!(2, cells.len());
        assert_eq!(75.0, cells[0].bounds().width);
        assert_eq!(25.0, cells[1].bounds().width);
    }
}

#[test]
fn raises_cell_prepared_events_on_initial_layout() {
    let _app = start_app();
    let t = create_target_with(Options { run_layout: false, ..Options::default() });
    let raised = counter();

    let counter = raised.clone();
    let _subscription = t.target.cell_prepared(move |_| increment(&counter));

    t.target.update_layout();

    assert_eq!(20, raised.get());
}

#[test]
fn raises_cell_clearing_cell_prepared_events_on_scroll() {
    let _app = start_app();
    let t = create_target();
    let clearing_raised = counter();
    let prepared_raised = counter();

    let clearing = clearing_raised.clone();
    let _clearing = t.target.cell_clearing(move |e: &TreeDataGridCellEventArgs| {
        assert_eq!(clearing.get() % 2, e.column_index());
        assert_eq!(0, e.row_index());
        increment(&clearing);
    });

    let prepared = prepared_raised.clone();
    let _prepared = t.target.cell_prepared(move |e: &TreeDataGridCellEventArgs| {
        assert_eq!(prepared.get() % 2, e.column_index());
        assert_eq!(10, e.row_index());
        increment(&prepared);
    });

    t.scroll().set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    assert_eq!(2, clearing_raised.get());
    assert_eq!(2, prepared_raised.get());
}

#[test]
fn raises_cell_value_changed_when_model_value_changed() {
    let _app = start_app();
    let t = create_target();
    let raised = counter();

    let counter = raised.clone();
    let _subscription = t.target.cell_value_changed(move |e: &TreeDataGridCellEventArgs| {
        assert_eq!(1, e.column_index());
        assert_eq!(1, e.row_index());
        increment(&counter);
    });

    t.item(1).set_title(Some("Changed".to_string()));

    assert_eq!(1, raised.get());
}

#[test]
fn raises_cell_value_changed_after_cell_edit() {
    let _app = start_app();
    let t = create_target();
    let raised = counter();

    let counter = raised.clone();
    let _subscription = t.target.cell_value_changed(move |e: &TreeDataGridCellEventArgs| {
        assert_eq!(1, e.column_index());
        assert_eq!(1, e.row_index());
        increment(&counter);
    });

    let cell = t
        .target
        .try_get_row(1)
        .and_then(|row| row.try_get_cell(1))
        .and_then(|cell| cell.cast::<TreeDataGridTextCell>())
        .expect("a text cell");
    cell.begin_edit();
    cell.set_value(Some("Changed".to_string()));

    assert_eq!(0, raised.get());
    assert_eq!(Some("Item 1".to_string()), t.item(1).title());

    cell.end_edit();

    assert_eq!(Some("Changed".to_string()), t.item(1).title());
    assert_eq!(1, raised.get());
}

#[test]
fn does_not_raise_cell_value_changed_events_on_initial_layout() {
    let _app = start_app();
    let t = create_target_with(Options { run_layout: false, ..Options::default() });
    let raised = counter();

    let counter = raised.clone();
    let _subscription = t.target.cell_value_changed(move |_| increment(&counter));

    t.target.update_layout();

    assert_eq!(0, raised.get());
}

#[test]
fn does_not_raise_cell_value_changed_events_on_scroll() {
    let _app = start_app();
    let t = create_target();
    let raised = counter();

    let counter = raised.clone();
    let _subscription = t.target.cell_value_changed(move |_| increment(&counter));

    t.scroll().set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    assert_eq!(0, raised.get());
}

#[test]
fn does_not_realize_columns_outside_viewport() {
    let _app = start_app();
    let t = create_target_with_columns(vec![
        id_column("ID", Some(stars(1.0)), None),
        title_column("Title1", None, min_width(50.0)),
        title_column("Title2", None, min_width(50.0)),
        title_column("Title3", None, min_width(50.0)),
    ]);

    assert_column_indexes(&t.target, 0, 3);

    let columns = t.target.columns().expect("columns");
    assert_eq!(30.0, columns.get(0).actual_width());
    assert_eq!(50.0, columns.get(1).actual_width());
    assert_eq!(50.0, columns.get(2).actual_width());
    assert_eq!(50.0, columns.get(3).actual_width());
}

#[test]
fn header_column_indexes_are_updated_when_columns_are_updated() {
    let _app = start_app();
    let t = create_target_with_columns(vec![
        id_column("ID", Some(stars(1.0)), None),
        title_column("Title1", Some(stars(1.0)), None),
        title_column("Title2", Some(stars(1.0)), None),
        title_column("Title3", Some(stars(1.0)), None),
    ]);

    assert_column_indexes(&t.target, 0, 4);

    let source = t.target.source().and_then(|source| source.downcast::<FlatTreeDataGridSource<Model>>()).expect("a flat source");
    assert!(Rc::ptr_eq(&t.source, &source));

    let moved_column = source.columns().get(1);
    source.columns().remove(&moved_column);

    let visual: &Visual = &t.target;
    let root = TopLevel::get_top_level(Some(visual)).expect("a top level");
    root.update_layout();
    run_jobs();

    assert_column_indexes(&t.target, 0, 3);

    source.columns().add(moved_column);

    root.update_layout();
    run_jobs();

    assert_column_indexes(&t.target, 0, 4);
}

#[test]
fn columns_are_correctly_sized_after_changing_source() {
    let _app = start_app();
    // Create the initial target with 2 columns and make sure our preconditions are correct.
    let t = create_target_with_columns(vec![id_column("ID", Some(stars(1.0)), None), title_column("Title1", None, min_width(50.0))]);

    assert_column_indexes(&t.target, 0, 2);

    // Create a new source and assign it to the TreeDataGrid.
    let new_columns = vec![
        id_column("ID", Some(stars(1.0)), None),
        title_column("Title1", None, min_width(20.0)),
        title_column("Title2", None, min_width(20.0)),
    ];
    let new_source = new_source(&t.items, &new_columns);

    t.target.set_source(source_ref::<Model, _>(&new_source));

    // The columns should not have an ActualWidth yet.
    assert!(new_columns[0].view.actual_width().is_nan());
    assert!(new_columns[1].view.actual_width().is_nan());
    assert!(new_columns[2].view.actual_width().is_nan());

    // Do a layout pass and check that the columns have been correctly sized.
    t.target.update_layout();
    assert_column_indexes(&t.target, 0, 3);

    let columns = t.target.columns().expect("columns");
    assert_eq!(60.0, columns.get(0).actual_width());
    assert_eq!(20.0, columns.get(1).actual_width());
    assert_eq!(20.0, columns.get(2).actual_width());
}

#[test]
fn should_correctly_align_columns_when_vertically_scrolling_with_first_column_unrealized() {
    // Issue #298
    fn assert_realized_cells(target: &TreeDataGrid) {
        let rows = visual_rows(target);

        for row in &rows {
            let cells = realized_cells_by_column(row);

            assert_eq!(3, cells.len());
            assert_eq!(1, cells[0].column_index());
            assert_eq!(100.0, cells[0].bounds().x);
            assert_eq!(150.0, cells[1].bounds().x);
            assert_eq!(200.0, cells[2].bounds().x);
        }
    }

    let _app = start_app();
    let t = create_target_with_columns(vec![
        id_column("ID", Some(pixels(100.0)), None),
        title_column("Title1", Some(pixels(50.0)), None),
        title_column("Title2", Some(pixels(50.0)), None),
        title_column("Title3", Some(pixels(50.0)), None),
    ]);

    // Scroll horizontally and check that the realized cells are positioned correctly.
    t.scroll().set_offset(Vector::new(120.0, 0.0));
    t.target.update_layout();
    assert_realized_cells(&t.target);

    // Scroll down a row and check that the realized cells are positioned correctly.
    t.scroll().set_offset(Vector::new(120.0, 10.0));
    t.target.update_layout();
    assert_realized_cells(&t.target);

    // Now scroll back vertically and check once more.
    t.scroll().set_offset(Vector::new(120.0, 0.0));
    t.target.update_layout();
    assert_realized_cells(&t.target);
}

#[test]
fn uses_row_presenter_viewport_when_replacing_source_while_scrolled() {
    fn sixteen_columns() -> Vec<Col> {
        let mut columns = vec![id_column("ID", None, None)];
        for i in 1..=15 {
            columns.push(title_column(&format!("Title{i}"), None, None));
        }
        columns
    }

    fn assert_realized_cells(
        target: &TreeDataGrid,
        assert: impl Fn(&[Ref<TreeDataGridColumnHeader>], &[Ref<TreeDataGridCell>]),
    ) {
        let mut column_headers: Vec<Ref<TreeDataGridColumnHeader>> = target
            .column_headers_presenter()
            .expect("a column headers presenter")
            .get_realized_elements()
            .into_iter()
            .map(|element| element.cast::<TreeDataGridColumnHeader>().expect("a column header"))
            .collect();
        column_headers.sort_by_key(|x| x.column_index());

        let rows: Vec<Ref<TreeDataGridRow>> = target
            .rows_presenter()
            .expect("a rows presenter")
            .get_realized_elements()
            .into_iter()
            .map(|element| element.cast::<TreeDataGridRow>().expect("a row"))
            .collect();

        for row in &rows {
            let cells = realized_cells_by_column(row);
            assert(&column_headers[..], &cells[..]);
        }
    }

    let _app = start_app();
    let t = create_target_with(Options {
        header_presenter_size: 10.0,
        columns: Some(sixteen_columns()),
        ..Options::default()
    });

    // Scroll all the way to the right.
    let scroll = t.scroll();
    scroll.set_offset(scroll.offset().with_x(scroll.extent().width));
    t.target.update_layout();
    run_jobs();

    // Replace the source. Unmeasured Auto columns reserve their minimum width while the retained
    // row viewport keeps the realized controls at the scrolled position.
    let new_columns = sixteen_columns();
    let new_source = new_source(&t.items, &new_columns);

    t.target.set_source(source_ref::<Model, _>(&new_source));
    run_jobs();

    assert_eq!(30.0, new_columns[1].view.actual_width());
    assert_eq!(30.0, new_columns[2].view.actual_width());
    assert_eq!(30.0, new_columns[3].view.actual_width());
    assert_eq!(30.0, new_columns[4].view.actual_width());

    for column in &new_columns[5..12] {
        assert_eq!(30.0, column.view.actual_width());
    }

    assert_realized_cells(&t.target, |column_headers, cells| {
        assert_eq!(4, cells.len());
        assert_eq!(12, cells[0].column_index());
        assert_eq!(13, cells[1].column_index());
        assert_eq!(14, cells[2].column_index());
        assert_eq!(15, cells[3].column_index());

        assert_eq!(12, column_headers[0].column_index());
        assert_eq!(13, column_headers[1].column_index());
        assert_eq!(14, column_headers[2].column_index());
        assert_eq!(15, column_headers[3].column_index());

        assert_eq!(column_headers[0].bounds().x, cells[0].bounds().x);
        assert_eq!(column_headers[1].bounds().x, cells[1].bounds().x);
        assert_eq!(column_headers[2].bounds().x, cells[2].bounds().x);
        assert_eq!(column_headers[3].bounds().x, cells[3].bounds().x);
    });
}

#[test]
fn should_use_text_cell_string_format() {
    let _app = start_app();
    let t = create_target_with_columns(vec![title_column("Title", None, string_format("Hello {0}"))]);

    let rows = visual_rows(&t.target);

    assert_eq!(10, rows.len());

    for (i, row) in rows.iter().enumerate() {
        let cells = visual_cells(row);
        assert_eq!(1, cells.len());
        let cell = cells[0].cast::<TreeDataGridTextCell>().expect("a text cell");
        assert_eq!(Some(format!("Hello Item {i}")), cell.value());
    }
}

#[test]
fn should_use_text_cell_string_format_when_model_is_updated() {
    let _app = start_app();
    let t = create_target_with_columns(vec![title_column("Title", None, string_format("Hello {0}"))]);

    let rows = visual_rows(&t.target);

    assert_eq!(10, rows.len());
    t.item(1).set_title(Some("World".to_string()));
    let cell = t.target.try_get_cell(0, 1).and_then(|cell| cell.cast::<TreeDataGridTextCell>()).expect("a text cell");

    assert_eq!(Some("Hello World".to_string()), cell.value());
}

/// `TreeDataGridTests_Flat.RemoveItems`.
mod remove_items {
    use super::*;

    #[test]
    fn can_remove_range_within_realized_elements() {
        let _app = start_app();
        let t = create_target();

        t.scroll().set_offset(Vector::new(0.0, 100.0));
        layout(&t.target);

        assert_row_indexes(&t.target, 10, 10);

        t.items.remove_range(12, 4);
        layout(&t.target);

        assert_row_indexes(&t.target, 10, 10);
        assert_eq!(Vector::new(0.0, 100.0), t.scroll().offset());
    }

    #[test]
    fn can_remove_range_within_realized_elements_when_scrolled_to_end() {
        let _app = start_app();
        let t = create_target_with(Options { item_count: 20, ..Options::default() });

        t.scroll().set_offset(Vector::new(0.0, 100.0));
        layout(&t.target);

        assert_row_indexes(&t.target, 10, 10);

        t.items.remove_range(12, 4);
        layout(&t.target);

        assert_row_indexes(&t.target, 6, 10);
        assert_eq!(Vector::new(0.0, 60.0), t.scroll().offset());
    }

    #[test]
    fn can_remove_range_of_all_realized_elements() {
        let _app = start_app();
        let t = create_target();

        t.scroll().set_offset(Vector::new(0.0, 100.0));
        layout(&t.target);

        assert_row_indexes(&t.target, 10, 10);

        t.items.remove_range(10, 10);
        layout(&t.target);

        assert_row_indexes(&t.target, 10, 10);
        assert_eq!(Vector::new(0.0, 100.0), t.scroll().offset());
    }

    #[test]
    fn can_remove_range_of_all_realized_elements_when_scrolled_to_end() {
        let _app = start_app();
        let t = create_target_with(Options { item_count: 20, ..Options::default() });

        t.scroll().set_offset(Vector::new(0.0, 100.0));
        layout(&t.target);

        assert_row_indexes(&t.target, 10, 10);

        t.items.remove_range(10, 10);
        layout(&t.target);

        assert_row_indexes(&t.target, 0, 10);
        assert_eq!(Vector::new(0.0, 0.0), t.scroll().offset());
    }

    #[test]
    fn can_remove_range_spanning_beginning_of_realized_elements_when_scrolled_to_end() {
        let _app = start_app();
        let t = create_target_with(Options { item_count: 20, ..Options::default() });

        t.scroll().set_offset(Vector::new(0.0, 100.0));
        layout(&t.target);

        assert_row_indexes(&t.target, 10, 10);

        t.items.remove_range(5, 10);
        layout(&t.target);

        assert_row_indexes(&t.target, 0, 10);
        assert_eq!(Vector::new(0.0, 0.0), t.scroll().offset());
    }

    #[test]
    fn can_remove_range_spanning_end_of_realized_elements() {
        let _app = start_app();
        let t = create_target();

        t.scroll().set_offset(Vector::new(0.0, 100.0));
        layout(&t.target);

        assert_row_indexes(&t.target, 10, 10);

        t.items.remove_range(15, 10);
        layout(&t.target);

        assert_row_indexes(&t.target, 10, 10);
        assert_eq!(Vector::new(0.0, 100.0), t.scroll().offset());
    }

    #[test]
    fn can_remove_selected_item() {
        let _app = start_app();
        let t = create_target();

        layout(&t.target);
        t.row_selection().select(&IndexPath::new(3));

        assert_eq!(IndexPath::new(3), t.row_selection().selected_index());

        t.items.remove_at(3);

        assert_eq!(IndexPath::new(-1), t.row_selection().selected_index());
    }

    #[test]
    fn can_remove_selected_item_sorted() {
        let _app = start_app();
        let t = create_target();
        sort_by_first_column(&t.target, ListSortDirection::Descending);

        layout(&t.target);
        t.row_selection().select(&IndexPath::new(3));

        assert_eq!(IndexPath::new(3), t.row_selection().selected_index());

        t.items.remove_at(3);

        assert_eq!(IndexPath::new(-1), t.row_selection().selected_index());
    }
}

/// The header scroll viewer of the template (`PART_HeaderScrollViewer`).
fn header_scroll_viewer(target: &TreeDataGrid) -> Ref<ScrollViewer> {
    let found: Vec<Ref<Visual>> = target
        .get_visual_descendants()
        .filter(|x| x.name().as_deref() == Some("PART_HeaderScrollViewer"))
        .collect();
    assert_eq!(1, found.len());
    found[0].cast::<ScrollViewer>().expect("a scroll viewer")
}

#[test]
fn should_show_horizontal_scroll_bar() {
    let _app = start_app();
    let t = create_target_with_columns(vec![
        id_column("ID", Some(pixels(100.0)), None),
        title_column("Title1", Some(pixels(100.0)), None),
    ]);
    let scroll = t.target.scroll().expect("a scroll viewer");
    let header_scroll = header_scroll_viewer(&t.target);

    assert_eq!(Size::new(100.0, 100.0), scroll.viewport());
    assert_eq!(Size::new(200.0, 1000.0), scroll.extent());
    assert_eq!(Size::new(100.0, 0.0), header_scroll.viewport());
    assert_eq!(Size::new(200.0, 0.0), header_scroll.extent());
}

#[test]
fn should_show_horizontal_scroll_bar_with_no_initial_rows() {
    let _app = start_app();
    let t = create_target_with(Options {
        columns: Some(vec![
            id_column("ID", Some(pixels(100.0)), None),
            title_column("Title1", Some(pixels(100.0)), None),
        ]),
        item_count: 0,
        ..Options::default()
    });
    let scroll = t.target.scroll().expect("a scroll viewer");
    let header_scroll = header_scroll_viewer(&t.target);

    assert_eq!(Size::new(100.0, 100.0), scroll.viewport());
    assert_eq!(Size::new(200.0, 100.0), scroll.extent());
    assert_eq!(Size::new(100.0, 0.0), header_scroll.viewport());
    assert_eq!(Size::new(200.0, 0.0), header_scroll.extent());
}

#[test]
fn should_preserve_horizontal_scroll_bar_when_rows_removed() {
    let _app = start_app();
    let t = create_target_with_columns(vec![
        id_column("ID", Some(pixels(100.0)), None),
        title_column("Title1", Some(pixels(100.0)), None),
    ]);
    let scroll = t.target.scroll().expect("a scroll viewer");
    let header_scroll = header_scroll_viewer(&t.target);

    // The reference subscribes an empty handler to the changes of the scroll viewer's `Extent`
    // here; it has no effect and is left out.
    t.items.clear();
    t.target.update_layout();

    assert_eq!(Size::new(100.0, 100.0), scroll.viewport());
    assert_eq!(Size::new(200.0, 100.0), scroll.extent());
    assert_eq!(Size::new(100.0, 0.0), header_scroll.viewport());
    assert_eq!(Size::new(200.0, 0.0), header_scroll.extent());
}
