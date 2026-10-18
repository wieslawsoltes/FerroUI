//! `CoreSourcePresentationTests.cs`: the presentation of the core's sources: the neutral columns
//! presented as view columns, sorting, selection and column state shared with the core, the
//! hierarchical projection, disposal and suspension of a presentation, the grid's `Model`, named
//! templates, custom view columns and the selection mode of the grid.
//!
//! The reference's `using var` disposals at the end of a test release objects the test no longer
//! reads; the port leaves them to be dropped. Its `try { ... } finally { window.Close(); }` closes
//! the window at the end of the test.

use super::support::{
    assert_panics, model_of, observable, presentation_of, run_jobs, same_model, source_ref, start_app, test_window,
    tree_data_grid_row_style, tree_data_grid_template, TestNotifier,
};
use crate::automation::peers::{TreeDataGridAutomationPeer, TreeDataGridRowAutomationPeer};
use crate::models::{
    CellRef, CheckBoxCell, ColumnRef, ICellColumn, IColumn, IUpdateColumnLayout,
    TemplateColumn,
};
use crate::presentation::{
    register_disposable_column, CoreTextColumn, IDisposableColumn, ITreeDataGridPresentationOptions,
    TreeDataGridPresentation, TreeDataGridPresentationOptions, TreeDataGridSourcePresentation,
};
use crate::primitives::TreeDataGridCell;
use crate::{TreeDataGrid, TreeDataGridRowDropPosition, TreeDataGridSelectionMode};
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::input::{DragDropEffects, InputElement, Key, KeyEventArgs};
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{ferro_model, BoxedValue, Vector};
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::{Control, GridLength, TextBlock};
use ferroui_treedatagrid_core::models::{
    self as core_models, ColumnList, ColumnRef as CoreColumnRef, IColumn as CoreColumn, IRows,
    ListSortDirection, RowRef, RowsChangedArgs, ValueColumn,
};
use ferroui_treedatagrid_core::selection::{IReadOnlyList, ITreeDataGridSelection, UntypedModels};
use ferroui_treedatagrid_core::{
    Accessor, FlatTreeDataGridSource, GridLength as CoreGridLength, HierarchicalTreeDataGridSource, ITreeDataGridSource,
    ITreeDataGridSourceOf, IndexPath, Models, RowDropPosition, RowMoveEffects,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// Models and helpers
// ---------------------------------------------------------------------------------------------

/// `Node`: a name, an expansion state the expander column writes and the children.
struct Node {
    name: RefCell<String>,
    expanded: Cell<bool>,
    children: Rc<FerroList<Rc<Node>>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Node {
    fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(name.to_string()), expanded: Cell::new(false), children: Rc::new(FerroList::new()) })
    }

    fn name(&self) -> String {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: String) {
        *self.name.borrow_mut() = value;
    }
}

/// `BoundValue`: a model that notifies the changes of its `Value`; its flags do not notify.
struct BoundValue {
    notifier: TestNotifier,
    value: RefCell<String>,
    flag: Cell<bool>,
    optional_flag: Cell<Option<bool>>,
}

impl BoundValue {
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            notifier: TestNotifier::new(),
            value: RefCell::new(String::new()),
            flag: Cell::new(false),
            optional_flag: Cell::new(Some(true)),
        })
    }

    fn value(&self) -> String {
        self.value.borrow().clone()
    }

    fn set_value(&self, value: String) {
        *self.value.borrow_mut() = value;
        self.notifier.raise("Value");
    }
}

impl INotifyPropertyChanged for BoundValue {
    fn property_changed(&self) -> &Event<str> {
        self.notifier.event()
    }
}

ferro_model!(BoundValue, |b| b.notify_property_changed());

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// Whether two shared values are the same object (`Assert.Same`).
fn same<A: ?Sized, B: ?Sized>(a: &Rc<A>, b: &Rc<B>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// `x => x.Name` with its setter.
fn name_accessor() -> Accessor<Node, String> {
    Accessor::member("Name", |x: &Rc<Node>| x.name()).with_setter(|x: &Rc<Node>, value: String| x.set_name(value))
}

/// `Flat(items)`: a flat source of `items` with a writable text column of the name.
fn flat(items: &Rc<FerroList<Rc<Node>>>) -> Rc<FlatTreeDataGridSource<Node>> {
    let source = FlatTreeDataGridSource::new(items.clone() as Models<Node>);
    source.columns().add(core_models::TextColumn::new(header("Name"), name_accessor(), None, None, None) as CoreColumnRef<Node>);
    source
}

/// The template of the "card" columns: a text block of the model's name.
fn card_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::for_type::<Node>(
        |model, _| {
            let text = TextBlock::new();
            let name = model.name();
            text.set_text(Some(name.as_str()));
            Some(text.upcast())
        },
        false,
    )
}

/// The options whose "card" columns are template columns of the card template.
fn card_options() -> Rc<TreeDataGridPresentationOptions> {
    let options = TreeDataGridPresentationOptions::new();
    options.set_column("card", |column: &Rc<dyn CoreColumn>| {
        TemplateColumn::<Node>::new(column.header(), card_template(), None, None, None) as ColumnRef
    });
    options
}

fn key_down(key: Key) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key = key;
    e
}

fn counter() -> (Rc<Cell<i32>>, Rc<Cell<i32>>) {
    let count = Rc::new(Cell::new(0));
    (count.clone(), count)
}

fn names(items: &FerroList<Rc<Node>>) -> Vec<String> {
    (0..items.count()).map(|i| items.get(i).name()).collect()
}

// ---------------------------------------------------------------------------------------------
// ReplaceableRowsSource
// ---------------------------------------------------------------------------------------------

/// `ReplaceableRowsSource`: a flat source whose `ITreeDataGridSource.Rows` can be replaced by other
/// rows. The reference derives from the flat source and reimplements `Rows`; the port wraps a
/// flat source and forwards everything but its rows to it.
struct ReplaceableRowsSource {
    inner: Rc<FlatTreeDataGridSource<Node>>,
    current_rows: RefCell<Rc<dyn IRows>>,
}

impl ReplaceableRowsSource {
    fn new(items: Models<Node>) -> Rc<Self> {
        let inner = FlatTreeDataGridSource::new(items);
        let current_rows = ITreeDataGridSource::rows(&*inner);
        inner.columns().add(
            core_models::TextColumn::new(header("Name"), Accessor::member("Name", |x: &Rc<Node>| x.name()), None, None, None)
                as CoreColumnRef<Node>,
        );
        Rc::new(Self { inner, current_rows: RefCell::new(current_rows) })
    }

    /// `ReplaceRows`: shows `rows` and raises the change of `Rows`.
    fn replace_rows(&self, rows: Rc<dyn IRows>) {
        *self.current_rows.borrow_mut() = rows;
        INotifyPropertyChanged::property_changed(&*self.inner).raise("Rows");
    }
}

impl INotifyPropertyChanged for ReplaceableRowsSource {
    fn property_changed(&self) -> &Event<str> {
        INotifyPropertyChanged::property_changed(&*self.inner)
    }
}

impl ITreeDataGridSource for ReplaceableRowsSource {
    fn column_count(&self) -> usize {
        ITreeDataGridSource::column_count(&*self.inner)
    }

    fn column(&self, index: usize) -> Rc<dyn CoreColumn> {
        ITreeDataGridSource::column(&*self.inner, index)
    }

    fn rows(&self) -> Rc<dyn IRows> {
        self.current_rows.borrow().clone()
    }

    fn selection(&self) -> Option<Rc<dyn ITreeDataGridSelection>> {
        ITreeDataGridSource::selection(&*self.inner)
    }

    fn set_selection(&self, value: Option<Rc<dyn ITreeDataGridSelection>>) {
        ITreeDataGridSource::set_selection(&*self.inner, value);
    }

    fn is_hierarchical(&self) -> bool {
        ITreeDataGridSource::is_hierarchical(&*self.inner)
    }

    fn is_sorted(&self) -> bool {
        ITreeDataGridSource::is_sorted(&*self.inner)
    }

    fn items_untyped(&self) -> UntypedModels {
        ITreeDataGridSource::items_untyped(&*self.inner)
    }

    fn get_model_children_untyped(&self, model: &BoxedValue) -> Option<Vec<BoxedValue>> {
        ITreeDataGridSource::get_model_children_untyped(&*self.inner, model)
    }

    fn sort_by(&self, column: Option<&dyn CoreColumn>, direction: ListSortDirection) -> bool {
        ITreeDataGridSource::sort_by(&*self.inner, column, direction)
    }

    fn clear_sort(&self) {
        ITreeDataGridSource::clear_sort(&*self.inner);
    }

    fn add_sorted(&self, handler: Rc<dyn Fn()>) -> u64 {
        ITreeDataGridSource::add_sorted(&*self.inner, handler)
    }

    fn remove_sorted(&self, token: u64) {
        ITreeDataGridSource::remove_sorted(&*self.inner, token);
    }

    fn move_rows(
        &self,
        source: &dyn ITreeDataGridSource,
        indexes: &[IndexPath],
        target_index: &IndexPath,
        position: RowDropPosition,
        effects: RowMoveEffects,
    ) {
        ITreeDataGridSource::move_rows(&*self.inner, source, indexes, target_index, position, effects);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ITreeDataGridSourceOf<Node> for ReplaceableRowsSource {
    fn items(&self) -> Models<Node> {
        ITreeDataGridSourceOf::items(&*self.inner)
    }

    fn set_items(&self, value: Models<Node>) {
        ITreeDataGridSourceOf::set_items(&*self.inner, value);
    }

    fn columns(&self) -> &ColumnList<Node> {
        ITreeDataGridSourceOf::columns(&*self.inner)
    }

    fn get_model_children(&self, model: &Rc<Node>) -> Option<Models<Node>> {
        ITreeDataGridSourceOf::get_model_children(&*self.inner, model)
    }
}

// ---------------------------------------------------------------------------------------------
// DisposableTemplateColumn
// ---------------------------------------------------------------------------------------------

/// `DisposableTemplateColumn`: a template column of the card template that records its disposal.
/// The reference derives from `TemplateColumn<Node>`; the port holds one and forwards the view
/// contracts to it.
struct DisposableTemplateColumn {
    inner: ColumnRef,
    is_disposed: Cell<bool>,
}

impl DisposableTemplateColumn {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            inner: TemplateColumn::<Node>::new(header("Card"), card_template(), None, None, None) as ColumnRef,
            is_disposed: Cell::new(false),
        })
    }

    fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    fn layout(&self) -> &dyn IUpdateColumnLayout {
        self.inner.as_update_column_layout().expect("a template column lays itself out")
    }

    fn cells(&self) -> &dyn ICellColumn {
        self.inner.as_cell_column().expect("a template column creates cells")
    }
}

impl IDisposableColumn for DisposableTemplateColumn {
    fn dispose(&self) {
        self.is_disposed.set(true);
    }
}

impl INotifyPropertyChanged for DisposableTemplateColumn {
    fn property_changed(&self) -> &Event<str> {
        self.inner.property_changed()
    }
}

impl IColumn for DisposableTemplateColumn {
    fn actual_width(&self) -> f64 {
        self.inner.actual_width()
    }

    fn can_user_resize(&self) -> Option<bool> {
        self.inner.can_user_resize()
    }

    fn header(&self) -> Option<BoxedValue> {
        self.inner.header()
    }

    fn width(&self) -> GridLength {
        self.inner.width()
    }

    fn sort_direction(&self) -> Option<ListSortDirection> {
        self.inner.sort_direction()
    }

    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.inner.set_sort_direction(value);
    }

    fn tag(&self) -> Option<BoxedValue> {
        self.inner.tag()
    }

    fn set_tag(&self, value: Option<BoxedValue>) {
        self.inner.set_tag(value);
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
        self.inner.requires_unconstrained_width_measurement()
    }
}

impl IUpdateColumnLayout for DisposableTemplateColumn {
    fn min_actual_width(&self) -> f64 {
        self.layout().min_actual_width()
    }

    fn max_actual_width(&self) -> f64 {
        self.layout().max_actual_width()
    }

    fn star_width_was_constrained(&self) -> bool {
        self.layout().star_width_was_constrained()
    }

    fn cell_measured(&self, width: f64, row_index: i32) -> f64 {
        self.layout().cell_measured(width, row_index)
    }

    fn calculate_star_width(&self, available_width: f64, total_stars: f64) {
        self.layout().calculate_star_width(available_width, total_stars);
    }

    fn commit_actual_width(&self) -> bool {
        self.layout().commit_actual_width()
    }

    fn set_width(&self, width: GridLength) {
        self.layout().set_width(width);
    }
}

impl ICellColumn for DisposableTemplateColumn {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        self.cells().create_cell(row)
    }

    fn try_reuse_cell(&self, cell: &CellRef, row: &RowRef) -> bool {
        self.cells().try_reuse_cell(cell, row)
    }

    fn select_value(&self, model: &BoxedValue) -> Option<String> {
        self.cells().select_value(model)
    }

    fn is_text_search_enabled(&self) -> bool {
        self.cells().is_text_search_enabled()
    }
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[test]
fn presentation_uses_neutral_sort_selection_and_column_state() {
    let _app = start_app();
    let items = observable(vec![Node::new("B"), Node::new("A")]);
    let source = flat(&items);
    let row_selection = source.row_selection().expect("a row selection");
    row_selection.base().set_selected_index(IndexPath::new(0));
    let presentation = presentation_of::<Node, _>(&source);

    let selected_item = row_selection.base().selected_item().expect("a selected item");
    assert!(same_model(&selected_item, presentation.selected_items().expect("selected items")[0].as_ref()));
    assert!(same(&ITreeDataGridSource::rows(&*source).get(0), &presentation.rows().get(0)));
    // `Assert.False(typeof(ITreeDataGridSource).IsAssignableFrom(presentation.GetType()))`: the
    // presentation is not a source itself (its type does not implement `ITreeDataGridSource`,
    // which Rust checks when it compiles); it presents the source it holds.
    assert!(same(&presentation.source_identity(), &source));
    assert!(presentation.columns().get(0).as_any().downcast_ref::<CoreTextColumn>().is_some());

    presentation.sort_by(&presentation.columns().get(0), ListSortDirection::Ascending);
    assert_eq!("A", model_of::<Node>(presentation.rows().get(0).model()).name());
    let selected_indexes = presentation.selected_indexes().expect("selected indexes");
    assert_eq!(1, presentation.rows().model_index_to_row_index(&selected_indexes[0]));

    presentation.select(&IndexPath::new(1), true);
    assert_eq!(IndexPath::new(1), row_selection.base().selected_index());
    row_selection.base().set_selected_index(IndexPath::new(0));
    assert_eq!(IndexPath::new(0), presentation.selected_indexes().expect("selected indexes")[0]);

    presentation.columns().set_column_width(0, GridLength::from_pixels(142.0));
    assert_eq!(CoreGridLength::pixels(142.0), source.columns().get(0).width());
    source.columns().get(0).set_width(CoreGridLength::STAR);
    assert_eq!(GridLength::STAR, presentation.columns().get(0).width());

    source.columns().get(0).set_is_visible(false);
    assert_eq!(0, presentation.columns().count());
    source.columns().get(0).set_is_visible(true);
    assert_eq!(1, presentation.columns().count());
}

#[test]
fn hierarchy_cells_expand_the_neutral_projection_and_update_models() {
    let _app = start_app();
    let parent = Node::new("Root");
    parent.children.add(Node::new("Child"));
    let source = HierarchicalTreeDataGridSource::new(Rc::new(vec![parent.clone()]) as Models<Node>);
    source.columns().add(core_models::HierarchicalExpanderColumn::new(
        core_models::TextColumn::new(header("Name"), Accessor::member("Name", |x: &Rc<Node>| x.name()), None, None, None)
            as CoreColumnRef<Node>,
        |x: &Rc<Node>| Some(x.children.clone() as Models<Node>),
        None,
        None,
        Some(Rc::new(|x: &Rc<Node>, expanded: bool| x.expanded.set(expanded)) as Rc<dyn Fn(&Rc<Node>, bool)>),
    ) as CoreColumnRef<Node>);
    let presentation = presentation_of::<Node, _>(&source);

    let cell = presentation.rows().realize_cell(&presentation.columns().get(0), 0, 0);
    let expander = cell.as_expander_cell().expect("an expander cell");
    expander.set_is_expanded(true);
    assert!(parent.expanded.get());
    assert_eq!(2, ITreeDataGridSource::rows(&*source).count());
    assert_eq!(2, presentation.rows().count());

    parent.children.add(Node::new("Second"));
    assert_eq!(3, presentation.rows().count());

    source.collapse(&IndexPath::new(0));
    assert!(!expander.is_expanded());
    assert_eq!(1, presentation.rows().count());
    presentation.rows().unrealize_cell(&cell, 0, 0);
}

#[test]
fn disposal_detaches_presentation_and_allows_reattachment() {
    let _app = start_app();
    let items = observable(vec![Node::new("A")]);
    let source = flat(&items);
    let presentation = presentation_of::<Node, _>(&source);
    let (selection_events, count) = counter();
    presentation
        .selection_interaction()
        .expect("a selection interaction")
        .add_selection_changed(Rc::new(move || count.set(count.get() + 1)));
    let row_selection = source.row_selection().expect("a row selection");

    row_selection.base().set_selected_index(IndexPath::new(0));
    assert_eq!(1, selection_events.get());

    presentation.dispose();
    presentation.dispose();
    row_selection.base().clear();
    items.add(Node::new("B"));
    assert_eq!(1, selection_events.get());

    let replacement = presentation_of::<Node, _>(&source);
    assert_eq!(2, replacement.rows().count());
    row_selection.base().set_selected_index(IndexPath::new(1));
    assert!(same_model(&items.get(1), replacement.selected_items().expect("selected items")[0].as_ref()));
}

#[test]
fn control_renders_edits_scrolls_and_navigates_a_neutral_source() {
    let _app = start_app();
    let items = observable((0..100).map(|x| Node::new(&format!("Row {x}"))));
    let source = flat(&items);
    let grid = TreeDataGrid::new();
    grid.set_model(source_ref::<Node, _>(&source));
    grid.set_template(Some(tree_data_grid_template(0.0)));
    let presentation = grid.presentation().expect("a presentation");
    let window = test_window(Some(grid.clone().upcast()), None);
    window.styles().add(tree_data_grid_row_style());
    window.styles().add(Style::with_setters(
        Selectors::of_type::<TreeDataGridCell>(),
        [Setter::new(Layoutable::height_property(), 10.0)],
    ));

    window.update_layout();
    run_jobs();
    assert!(!grid.rows_presenter().expect("a rows presenter").get_realized_elements().is_empty());

    let row_selection = source.row_selection().expect("a row selection");
    row_selection.base().set_selected_index(IndexPath::new(0));
    presentation.selection_interaction().expect("a selection interaction").on_key_down(&grid, &key_down(Key::Down));
    assert_eq!(IndexPath::new(1), row_selection.base().selected_index());

    let cell = presentation.rows().realize_cell(&presentation.columns().get(0), 0, 1);
    cell.as_text_cell().expect("a text cell").set_text(Some("Edited".to_string()));
    assert_eq!("Edited", items.get(1).name());
    presentation.rows().unrealize_cell(&cell, 0, 1);

    grid.scroll().expect("a scroll viewer").set_offset(Vector::new(0.0, 400.0));
    window.update_layout();
    run_jobs();
    assert!(!grid.rows_presenter().expect("a rows presenter").get_realized_elements().is_empty());

    items.insert(0, Node::new("Inserted"));
    window.update_layout();
    run_jobs();
    assert_eq!(IndexPath::new(2), row_selection.base().selected_index());

    window.close();
}

#[test]
fn model_binding_suspends_subscriptions_and_retains_view_state_after_detach() {
    let _app = start_app();
    let source = flat(&observable(vec![Node::new("A")]));
    let grid = TreeDataGrid::new();
    grid.set_model(source_ref::<Node, _>(&source));
    grid.set_template(Some(tree_data_grid_template(0.0)));
    let window = test_window(Some(grid.clone().upcast()), None);
    window.styles().add(tree_data_grid_row_style());

    window.update_layout();
    assert!(grid.source().is_none());
    let first = grid.presentation().expect("a presentation");
    let (row_changes, count) = counter();
    first.rows().add_collection_changed(Rc::new(move |_: &RowsChangedArgs<'_>| count.set(count.get() + 1)));

    window.set_content(None);
    assert!(same(&first, &grid.presentation().expect("the presentation")));
    assert!(grid.presentation().expect("the presentation").selection_interaction().is_none());
    assert!(grid.model().expect("the model").is(&(source.clone() as Rc<dyn ITreeDataGridSource>)));
    source.columns().get(0).set_width(CoreGridLength::pixels(175.0));
    source.set_items(observable(vec![Node::new("Changed"), Node::new("New")]) as Models<Node>);
    let row_selection = source.row_selection().expect("a row selection");
    row_selection.base().set_selected_index(IndexPath::new(0));
    assert_eq!(0, row_changes.get());

    window.set_content(Some(Control::boxed(&grid)));
    window.update_layout();
    assert!(grid.presentation().is_some());
    assert!(same(&first, &grid.presentation().expect("the presentation")));
    let columns = grid.columns().expect("columns");
    assert_eq!(GridLength::from_pixels(175.0), columns.get(0).width());
    assert_eq!(2, grid.rows().expect("rows").count());
    assert_eq!(175.0, columns.get(0).actual_width());

    source.set_items(observable(vec![Node::new("Third")]) as Models<Node>);
    assert_eq!(1, row_changes.get());
    row_selection.base().set_selected_index(IndexPath::new(0));
    let selected_item = row_selection.base().selected_item().expect("a selected item");
    let selected_items = grid.presentation().expect("the presentation").selected_items().expect("selected items");
    assert!(same_model(&selected_item, selected_items[0].as_ref()));

    // The explicit legacy Source API still takes precedence when assigned (the reference's classic
    // flat source is the core's flat source in the port).
    let legacy = FlatTreeDataGridSource::new(Rc::new(Vec::<Rc<Node>>::new()) as Models<Node>);
    grid.set_source(source_ref::<Node, _>(&legacy));
    assert!(grid.model().is_none());
    assert!(grid.source().expect("the source").is(&(legacy.clone() as Rc<dyn ITreeDataGridSource>)));

    window.close();
}

#[test]
fn model_assigned_while_unattached_is_suspended_until_attached() {
    let _app = start_app();
    let items = observable(vec![Node::new("A")]);
    let source = flat(&items);
    let grid = TreeDataGrid::new();
    grid.set_model(source_ref::<Node, _>(&source));
    grid.set_template(Some(tree_data_grid_template(0.0)));
    let presentation = grid.presentation().expect("a presentation");
    let (row_changes, count) = counter();
    presentation.rows().add_collection_changed(Rc::new(move |_: &RowsChangedArgs<'_>| count.set(count.get() + 1)));

    assert!(presentation.selection_interaction().is_none());
    items.add(Node::new("B"));
    assert_eq!(0, row_changes.get());

    let window = test_window(Some(grid.clone().upcast()), None);
    window.styles().add(tree_data_grid_row_style());
    window.update_layout();
    assert!(presentation.selection_interaction().is_some());
    assert_eq!(2, presentation.rows().count());
    items.add(Node::new("C"));
    assert_eq!(1, row_changes.get());

    window.close();
}

#[test]
fn clearing_model_clears_the_core_presentation() {
    let _app = start_app();
    let source = flat(&observable(vec![Node::new("A")]));
    let grid = TreeDataGrid::new();
    grid.set_model(source_ref::<Node, _>(&source));
    assert!(grid.presentation().is_some());

    grid.set_model(None);

    assert!(grid.presentation().is_none());
    assert!(grid.columns().is_none());
    assert!(grid.rows().is_none());
}

#[test]
fn presentation_key_changed_while_detached_is_applied_on_reattach() {
    let _app = start_app();
    let source = flat(&observable(vec![Node::new("A")]));
    let options = card_options();
    let grid = TreeDataGrid::new();
    grid.set_model(source_ref::<Node, _>(&source));
    grid.set_presentation_options(Some(options.clone() as Rc<dyn ITreeDataGridPresentationOptions>));
    grid.set_template(Some(tree_data_grid_template(0.0)));
    let window = test_window(Some(grid.clone().upcast()), None);
    window.styles().add(tree_data_grid_row_style());

    window.update_layout();
    assert!(grid.columns().expect("columns").get(0).as_any().is::<CoreTextColumn>());
    window.set_content(None);
    source.columns().get(0).set_presentation_key(Some("card".to_string()));
    assert!(grid.columns().expect("columns").get(0).as_any().is::<CoreTextColumn>());
    window.set_content(Some(Control::boxed(&grid)));
    window.update_layout();
    assert!(grid.columns().expect("columns").get(0).as_any().is::<TemplateColumn<Node>>());

    window.close();
}

#[test]
fn views_share_core_state_but_keep_layout_and_cell_objects_separate() {
    let _app = start_app();
    let source = flat(&observable(vec![Node::new("A")]));
    let second = core_models::TextColumn::new(header("Other"), Accessor::member("Name", |x: &Rc<Node>| x.name()), None, None, None);
    source.columns().add(second as CoreColumnRef<Node>);
    let first_view = presentation_of::<Node, _>(&source);
    let second_view = presentation_of::<Node, _>(&source);

    let first_cell_column = first_view.columns().get(0);
    assert!(!same(&first_cell_column, &second_view.columns().get(0)));

    // The reference compares the typed bindings of the two views' text columns
    // (`TextColumn<Node, string>.Binding` and its `Links`) and overrides the first one's `Read`.
    // The port's text column of a neutral column binds each cell through the core column, so the
    // per-view state is the column's cell options: the two views hold distinct options over the
    // same core column, and a change of the first view's options (a format that replaces the
    // text) does not change what the second view's cells read.
    let first_text_column = first_cell_column.as_any().downcast_ref::<CoreTextColumn>().expect("a text column");
    let second_column = second_view.columns().get(0);
    let second_text_column = second_column.as_any().downcast_ref::<CoreTextColumn>().expect("a text column");
    assert!(!same(first_text_column.options(), second_text_column.options()));
    assert!(same(first_text_column.model(), second_text_column.model()));
    first_text_column.options().set_string_format("View override");
    let first_cell = first_view.rows().realize_cell(&first_cell_column, 0, 0);
    let second_cell = second_view.rows().realize_cell(&second_column, 0, 0);
    assert_eq!(Some("View override".to_string()), first_cell.as_text_cell().expect("a text cell").text());
    assert_eq!(Some("A".to_string()), second_cell.as_text_cell().expect("a text cell").text());
    first_view.rows().unrealize_cell(&first_cell, 0, 0);
    second_view.rows().unrealize_cell(&second_cell, 0, 0);

    let source_rows = ITreeDataGridSource::rows(&*source);
    assert!(same(&source_rows.get(0), &first_view.rows().get(0)));
    assert!(same(&source_rows.get(0), &second_view.rows().get(0)));

    first_view.columns().set_column_width(0, GridLength::from_pixels(150.0));
    assert_eq!(CoreGridLength::pixels(150.0), source.columns().get(0).width());
    assert_eq!(GridLength::from_pixels(150.0), second_view.columns().get(0).width());

    source.columns().move_column(0, 1);
    assert!(same(&first_cell_column, &first_view.columns().get(1)));
    source.columns().get(1).set_is_visible(false);
    source.columns().get(1).set_is_visible(true);
    assert!(same(&first_cell_column, &first_view.columns().get(1)));
}

#[test]
fn row_notifications_use_core_arguments_and_stop_on_disposal() {
    let _app = start_app();
    let items = observable(vec![Node::new("A")]);
    let source = flat(&items);
    // The arguments are compared by address: the presentation's rows raise the core's arguments
    // themselves.
    let model_args = Rc::new(Cell::new(0usize));
    let recorded = model_args.clone();
    ITreeDataGridSource::rows(&*source).add_collection_changed(Rc::new(move |args: &RowsChangedArgs<'_>| {
        recorded.set(args as *const RowsChangedArgs<'_> as *const () as usize);
    }));
    let view = presentation_of::<Node, _>(&source);
    let (calls, count) = counter();
    let expected = model_args.clone();
    view.rows().add_collection_changed(Rc::new(move |args: &RowsChangedArgs<'_>| {
        count.set(count.get() + 1);
        assert_eq!(expected.get(), args as *const RowsChangedArgs<'_> as *const () as usize);
    }));

    items.add(Node::new("B"));
    assert_eq!(1, calls.get());

    view.dispose();
    items.add(Node::new("C"));
    assert_eq!(1, calls.get());
}

#[test]
fn presentation_retargets_its_row_adapter_when_model_rows_are_replaced() {
    let _app = start_app();
    let old_items = observable(vec![Node::new("Old")]);
    let new_items = observable(vec![Node::new("New")]);
    let source = ReplaceableRowsSource::new(old_items.clone() as Models<Node>);
    let replacement = flat(&new_items);
    let presentation = TreeDataGridSourcePresentation::of::<Node, ReplaceableRowsSource>(source.clone(), None);
    let rows = presentation.rows();
    let (changes, count) = counter();
    rows.add_collection_changed(Rc::new(move |_: &RowsChangedArgs<'_>| count.set(count.get() + 1)));

    source.replace_rows(ITreeDataGridSource::rows(&*replacement));

    assert!(same(&rows, &presentation.rows()));
    assert_eq!("New", model_of::<Node>(rows.get(0).model()).name());
    assert_eq!(1, changes.get());
    old_items.add(Node::new("Ignored"));
    assert_eq!(1, changes.get());
    new_items.add(Node::new("Second"));
    assert_eq!(2, changes.get());
    assert_eq!(2, rows.count());
}

#[test]
fn delegate_columns_observe_the_model_and_edit_without_expression_compilation() {
    let _app = start_app();
    let item = BoundValue::new();
    item.set_value("A".to_string());
    let source = FlatTreeDataGridSource::new(Rc::new(vec![item.clone()]) as Models<BoundValue>);
    let column = ValueColumn::<BoundValue, String>::from_delegate(
        header("Value"),
        |x: &Rc<BoundValue>| x.value(),
        Some("Value"),
        Some(Rc::new(|x: &Rc<BoundValue>, value: String| x.set_value(value)) as Rc<dyn Fn(&Rc<BoundValue>, String)>),
        None,
        None,
        None,
    );
    source.columns().add(column.clone() as CoreColumnRef<BoundValue>);
    // `Assert.Same(getter, column.Getter)` and `Assert.Null(column.GetterExpression)`: the column
    // reads with the delegate it was given, named by the member it was given, and has no path of
    // an expression below the model.
    assert_eq!("A", column.accessor().get(&item));
    assert_eq!(Some("Value"), column.accessor().property_name());
    assert!(column.accessor().links().is_empty());

    let view = presentation_of::<BoundValue, _>(&source);
    let cell = view.rows().realize_cell(&view.columns().get(0), 0, 0);
    let text = cell.as_text_cell().expect("a text cell");
    assert_eq!(Some("A".to_string()), text.text());
    item.set_value("B".to_string());
    assert_eq!(Some("B".to_string()), text.text());
    text.set_text(Some("C".to_string()));
    assert_eq!("C", item.value());
    view.rows().unrealize_cell(&cell, 0, 0);
}

#[test]
fn checkbox_presentations_preserve_two_and_three_state_editing() {
    let _app = start_app();
    let item = BoundValue::new();
    let source = FlatTreeDataGridSource::new(Rc::new(vec![item.clone()]) as Models<BoundValue>);
    source.columns().add(core_models::CheckBoxColumn::new(
        header("Two"),
        Accessor::member("Flag", |x: &Rc<BoundValue>| x.flag.get()).with_setter(|x: &Rc<BoundValue>, value: bool| x.flag.set(value)),
        None,
        None,
        None,
    ) as CoreColumnRef<BoundValue>);
    source.columns().add(core_models::CheckBoxColumn::new_three_state(
        header("Three"),
        Accessor::member("OptionalFlag", |x: &Rc<BoundValue>| x.optional_flag.get())
            .with_setter(|x: &Rc<BoundValue>, value: Option<bool>| x.optional_flag.set(value)),
        None,
        None,
        None,
    ) as CoreColumnRef<BoundValue>);
    let view = presentation_of::<BoundValue, _>(&source);

    let two_cell = view.rows().realize_cell(&view.columns().get(0), 0, 0);
    let three_cell = view.rows().realize_cell(&view.columns().get(1), 1, 0);
    let two = two_cell.as_any().downcast_ref::<CheckBoxCell>().expect("a check box cell");
    let three = three_cell.as_any().downcast_ref::<CheckBoxCell>().expect("a check box cell");
    assert!(!two.is_three_state());
    assert!(three.is_three_state());

    two.set_value(Some(true));
    three.set_value(None);
    assert!(item.flag.get());
    assert_eq!(None, item.optional_flag.get());

    two_cell.dispose();
    three_cell.dispose();
}

#[test]
fn automation_selects_core_rows_and_moves_use_the_core_source() {
    let _app = start_app();
    let items = observable(vec![Node::new("A"), Node::new("B"), Node::new("C")]);
    let source = flat(&items);
    let grid = TreeDataGrid::new();
    grid.set_model(source_ref::<Node, _>(&source));
    grid.set_template(Some(tree_data_grid_template(0.0)));
    let window = test_window(Some(grid.clone().upcast()), None);
    window.styles().add(tree_data_grid_row_style());

    window.update_layout();
    let peer = TreeDataGridAutomationPeer::new(&grid);
    let row_control = grid.try_get_row(1).expect("a realized row");
    let row = TreeDataGridRowAutomationPeer::new(&row_control);
    assert!(row.select().is_ok());
    let row_selection = source.row_selection().expect("a row selection");
    assert_eq!(IndexPath::new(1), row_selection.base().selected_index());
    assert!(row.is_selected());
    assert_eq!(1, peer.get_selection().len());

    assert!(row.remove_from_selection().is_ok());
    assert_eq!(0, row_selection.base().selected_indexes().count());

    grid.presentation().expect("a presentation").move_rows(
        &[IndexPath::new(0)],
        &IndexPath::new(2),
        TreeDataGridRowDropPosition::After,
        DragDropEffects::MOVE,
    );
    assert_eq!(vec!["B".to_string(), "C".to_string(), "A".to_string()], names(&items));

    window.close();
}

#[test]
fn named_templates_are_view_factories_over_core_rows() {
    let _app = start_app();
    let source = FlatTreeDataGridSource::new(Rc::new(vec![Node::new("A")]) as Models<Node>);
    source.columns().add(
        core_models::TemplateColumn::new::<Node>(header("Card"), "card", Some(CoreGridLength::pixels(125.0)), None, None)
            as CoreColumnRef<Node>,
    );
    let options = card_options();
    let view = TreeDataGridSourcePresentation::of::<Node, _>(source.clone(), Some(options));

    assert!(view.columns().get(0).as_any().is::<TemplateColumn<Node>>());
    assert_eq!(GridLength::from_pixels(125.0), view.columns().get(0).width());
    let cell = view.rows().realize_cell(&view.columns().get(0), 0, 0);
    assert!(same_model(&source.items().get(0), cell.value().as_ref()));
    view.rows().unrealize_cell(&cell, 0, 0);
}

#[test]
fn expander_presentation_disposes_its_custom_inner_column() {
    let _app = start_app();
    register_disposable_column::<DisposableTemplateColumn>();
    let source = HierarchicalTreeDataGridSource::new(Rc::new(vec![Node::new("A")]) as Models<Node>);
    source.columns().add(core_models::HierarchicalExpanderColumn::new(
        core_models::TemplateColumn::new::<Node>(header("Card"), "disposable", None, None, None) as CoreColumnRef<Node>,
        |x: &Rc<Node>| Some(x.children.clone() as Models<Node>),
        None,
        None,
        None,
    ) as CoreColumnRef<Node>);
    let options = TreeDataGridPresentationOptions::new();
    let inner: Rc<RefCell<Option<Rc<DisposableTemplateColumn>>>> = Rc::new(RefCell::new(None));
    let created = inner.clone();
    options.set_column("disposable", move |_: &Rc<dyn CoreColumn>| {
        let column = DisposableTemplateColumn::new();
        *created.borrow_mut() = Some(column.clone());
        column as ColumnRef
    });

    let view = TreeDataGridSourcePresentation::of::<Node, _>(source.clone(), Some(options));
    view.dispose();

    let inner = inner.borrow().clone().expect("the inner column was created");
    assert!(inner.is_disposed());
}

#[test]
fn failed_presentation_construction_disposes_columns_created_before_the_failure() {
    let _app = start_app();
    register_disposable_column::<DisposableTemplateColumn>();
    let source = FlatTreeDataGridSource::new(Rc::new(vec![Node::new("A")]) as Models<Node>);
    source.columns().add(core_models::TemplateColumn::new::<Node>(header("First"), "disposable", None, None, None) as CoreColumnRef<Node>);
    source.columns().add(core_models::TemplateColumn::new::<Node>(header("Second"), "missing", None, None, None) as CoreColumnRef<Node>);
    let options = TreeDataGridPresentationOptions::new();
    let first: Rc<RefCell<Option<Rc<DisposableTemplateColumn>>>> = Rc::new(RefCell::new(None));
    let created = first.clone();
    options.set_column("disposable", move |_: &Rc<dyn CoreColumn>| {
        let column = DisposableTemplateColumn::new();
        *created.borrow_mut() = Some(column.clone());
        column as ColumnRef
    });

    assert_panics(|| {
        TreeDataGridSourcePresentation::of::<Node, _>(source.clone(), Some(options.clone()));
    });

    let first = first.borrow().clone().expect("the first column was created");
    assert!(first.is_disposed());
}

#[test]
fn v12_selection_mode_and_events_use_the_neutral_selection() {
    let _app = start_app();
    let source = flat(&observable(vec![Node::new("A"), Node::new("B")]));
    ITreeDataGridSource::set_selection(&*source, None);
    let grid = TreeDataGrid::new();
    grid.set_selection_mode(TreeDataGridSelectionMode::ROW | TreeDataGridSelectionMode::MULTIPLE);
    grid.set_model(source_ref::<Node, _>(&source));
    let row_selection = source.row_selection().expect("a row selection");
    assert!(!row_selection.base().single_select());
    let (event_count, count) = counter();
    let _subscription = grid.selection_changed(move |e| {
        count.set(count.get() + 1);
        assert_eq!(1, e.selected_indexes().len());
        assert_eq!("B", model_of::<Node>(e.selected_items()[0].clone()).name());
    });
    let window = test_window(Some(grid.clone().upcast()), None);

    row_selection.base().set_selected_index(IndexPath::new(1));
    assert_eq!(1, event_count.get());
    grid.set_selection_mode(TreeDataGridSelectionMode::ROW);
    assert!(row_selection.base().single_select());
    grid.set_model(None);

    window.close();
}
