//! `TreeDataGridV12Tests.cs`: the grid of an `ItemsSource` and declarative columns generates a
//! flat or hierarchical source that follows the items; the selection mode and the control's
//! `SelectionChanged`; the subscriptions of the grid scoped to its attachment; the declarative
//! columns read and write through their bindings (reflection and compiled, nested paths); and the
//! fluent column helpers of the sources.
//!
//! The items of the reference's tests are a notifying list of plain objects; here they are a
//! notifying list of untyped values ([`FerroList`] of `BoxedValue`) holding models whose types are
//! registered with `ferro_model!`, which the declarative columns read through the property
//! metadata of the types. The reference's expressions (`x => x.Name`) are accessors, which state
//! the setter the reference infers from the expression.

use super::support::{model_of, observable, run_jobs, start_app, tree_data_grid_row_style, tree_data_grid_with_template, visual_children_of};
use crate::models::{
    CheckBoxCell, CheckBoxColumn, IColumns, IExpanderCellPresentation, ITreeDataGridRows,
    TextColumn,
};
use crate::primitives::{TreeDataGridCell, TreeDataGridRow};
use crate::{
    HierarchicalTreeDataGridSourceExtensions, TreeDataGrid, TreeDataGridCheckBoxColumn, TreeDataGridColumn,
    TreeDataGridHierarchicalExpanderColumn, TreeDataGridSelectionChangedEventArgs, TreeDataGridSelectionMode,
    TreeDataGridSourceExtensions, TreeDataGridSourceRef, TreeDataGridTextColumn,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::plugins::PropertyInfoAccessorFactory;
use ferroui_base::data::core::{ClrPropertyInfo, Maybe, ModelRef, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingBase, CompiledBinding, CompiledBindingPathBuilder, ReflectionBinding};
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Control, ItemsSource, Window};
use ferroui_treedatagrid_core::models::{
    AnonymousRow, ColumnList, ColumnRef as CoreColumnRef, IColumn as CoreColumn, IRows, ListSortDirection, RowRef,
    TextColumn as NeutralTextColumn, ValueColumn,
};
use ferroui_treedatagrid_core::selection::{ITreeDataGridSelection, UntypedModels};
use ferroui_treedatagrid_core::{
    Accessor, CellIndex, FlatTreeDataGridSource, HierarchicalTreeDataGridSource, ITreeDataGridSource,
    ITreeDataGridSourceOf, IndexPath, Models, RowDropPosition, RowMoveEffects,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------------------------

/// `TestRow`: a plain model (no change notifications) with a name, an age, flags and children.
struct TestRow {
    name: RefCell<Option<String>>,
    age: Cell<i32>,
    is_active: Cell<bool>,
    is_optional: Cell<Option<bool>>,
    is_expanded: Cell<bool>,
    // `ObservableCollection<TestRow> Children`: the children as untyped values, the items of the
    // generated source's child rows.
    children: Rc<FerroList<BoxedValue>>,
}

ferroui_base::ferro_model!(TestRow, |b| b
    .property::<Maybe<String>>("Name", |o| o.name.borrow().clone(), |o, v| *o.name.borrow_mut() = v)
    .property::<Value<i32>>("Age", |o| o.age.get(), |o, v| o.age.set(v))
    .property::<Value<bool>>("IsActive", |o| o.is_active.get(), |o, v| o.is_active.set(v))
    .property::<Maybe<bool>>("IsOptional", |o| o.is_optional.get(), |o, v| o.is_optional.set(v))
    .property::<Value<bool>>("IsExpanded", |o| o.is_expanded.get(), |o, v| o.is_expanded.set(v))
    .read_only::<Value<bool>>("HasChildren", |o| o.children.count() > 0)
    .read_only::<Value<ItemsSource>>("Children", |o| ItemsSource::from(o.children.clone())));

impl TestRow {
    /// `new TestRow()`.
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            name: RefCell::new(None),
            age: Cell::new(0),
            is_active: Cell::new(false),
            is_optional: Cell::new(None),
            is_expanded: Cell::new(false),
            children: Rc::new(FerroList::new()),
        })
    }

    /// `new TestRow { Name = name }`.
    fn named(name: &str) -> Rc<Self> {
        let row = Self::new();
        row.set_name(Some(name.to_string()));
        row
    }

    fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    fn set_name(&self, value: Option<String>) {
        *self.name.borrow_mut() = value;
    }

    /// `Children = { child }`.
    fn add_child(&self, child: &Rc<TestRow>) {
        self.children.add(child.clone() as BoxedValue);
    }
}

/// The children of a row as the models of a typed source (`x => x.Children`).
fn typed_children(row: &Rc<TestRow>) -> Option<Models<TestRow>> {
    let children = row.children.to_vec().into_iter().map(|child| model_of::<TestRow>(Some(child)));
    Some(observable(children) as Models<TestRow>)
}

/// `NotifyingTestRow`: a customer, with change notifications.
struct NotifyingTestRow {
    customer: RefCell<Rc<CustomerInfo>>,
    changed: Event<str>,
}

ferroui_base::ferro_model!(NotifyingTestRow, |b| b.notify_property_changed().property::<ModelRef<CustomerInfo>>(
    "Customer",
    |o| Some(o.customer()),
    |o, v| {
        if let Some(v) = v {
            o.set_customer(v);
        }
    }
));

impl INotifyPropertyChanged for NotifyingTestRow {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl NotifyingTestRow {
    fn new(customer: Rc<CustomerInfo>) -> Rc<Self> {
        Model::new_model(Self { customer: RefCell::new(customer), changed: Event::new() })
    }

    fn customer(&self) -> Rc<CustomerInfo> {
        self.customer.borrow().clone()
    }

    fn set_customer(&self, value: Rc<CustomerInfo>) {
        if Rc::ptr_eq(&*self.customer.borrow(), &value) {
            return;
        }
        *self.customer.borrow_mut() = value;
        self.changed.raise("Customer");
    }
}

/// `CustomerInfo`: an address, with change notifications.
struct CustomerInfo {
    address: RefCell<Rc<AddressInfo>>,
    changed: Event<str>,
}

ferroui_base::ferro_model!(CustomerInfo, |b| b.notify_property_changed().property::<ModelRef<AddressInfo>>(
    "Address",
    |o| Some(o.address()),
    |o, v| {
        if let Some(v) = v {
            o.set_address(v);
        }
    }
));

impl INotifyPropertyChanged for CustomerInfo {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl CustomerInfo {
    fn new(address: Rc<AddressInfo>) -> Rc<Self> {
        Model::new_model(Self { address: RefCell::new(address), changed: Event::new() })
    }

    fn address(&self) -> Rc<AddressInfo> {
        self.address.borrow().clone()
    }

    fn set_address(&self, value: Rc<AddressInfo>) {
        if Rc::ptr_eq(&*self.address.borrow(), &value) {
            return;
        }
        *self.address.borrow_mut() = value;
        self.changed.raise("Address");
    }
}

/// `AddressInfo`: a city, with change notifications.
struct AddressInfo {
    city: RefCell<Option<String>>,
    changed: Event<str>,
}

ferroui_base::ferro_model!(AddressInfo, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("City", |o| o.city(), |o, v| o.set_city(v)));

impl INotifyPropertyChanged for AddressInfo {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl AddressInfo {
    fn new(city: &str) -> Rc<Self> {
        Model::new_model(Self { city: RefCell::new(Some(city.to_string())), changed: Event::new() })
    }

    fn city(&self) -> Option<String> {
        self.city.borrow().clone()
    }

    /// `City = value` (`SetAndRaise`).
    fn set_city(&self, value: Option<String>) {
        if *self.city.borrow() == value {
            return;
        }
        *self.city.borrow_mut() = value;
        self.changed.raise("City");
    }
}

/// `new NotifyingTestRow { Customer = { Address = { City = "Warsaw" } } }`.
fn notifying_row_in_warsaw() -> Rc<NotifyingTestRow> {
    NotifyingTestRow::new(CustomerInfo::new(AddressInfo::new("Warsaw")))
}

// ---------------------------------------------------------------------------------------------
// TrackingSource
// ---------------------------------------------------------------------------------------------

/// `TrackingSource`: a flat source of [`TestRow`]s that counts the subscribers to its own
/// `PropertyChanged` and `Sorted`, and to the changes of its column.
///
/// The reference wraps the source's view column list (`TrackingColumns`) to count the subscribers
/// to `LayoutInvalidated`. The port's sources hold core columns, and the view columns that lay
/// them out belong to the grid's presentation; what the grid holds of a source's columns is its
/// subscription to the changes of each core column, which is what this source counts. Its column
/// is a neutral text column of the core (the reference's `TextColumn<TestRow, string?>`).
struct TrackingSource {
    inner: Rc<FlatTreeDataGridSource<TestRow>>,
    column: Rc<ValueColumn<TestRow, Option<String>>>,
    property_changed: Event<str>,
    sorted: HandlerList<dyn Fn()>,
}

impl TrackingSource {
    fn new(items: Vec<Rc<TestRow>>) -> Rc<Self> {
        let inner = FlatTreeDataGridSource::new(observable(items) as Models<TestRow>);
        let column = NeutralTextColumn::new(
            header("Name"),
            Accessor::member("Name", |x: &Rc<TestRow>| x.name()),
            None,
            None,
            None,
        );
        inner.columns().add(column.clone() as CoreColumnRef<TestRow>);
        Rc::new(Self { inner, column, property_changed: Event::new(), sorted: HandlerList::new() })
    }

    /// `PropertyChangedSubscriberCount`.
    fn property_changed_subscriber_count(&self) -> usize {
        self.property_changed.handler_count()
    }

    /// `SortedSubscriberCount`.
    fn sorted_subscriber_count(&self) -> usize {
        self.sorted.len()
    }

    /// The subscribers to the changes of the source's column.
    fn column_subscriber_count(&self) -> usize {
        INotifyPropertyChanged::property_changed(&*self.column).handler_count()
    }
}

impl INotifyPropertyChanged for TrackingSource {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl ITreeDataGridSource for TrackingSource {
    fn column_count(&self) -> usize {
        ITreeDataGridSource::column_count(&*self.inner)
    }

    fn column(&self, index: usize) -> Rc<dyn CoreColumn> {
        ITreeDataGridSource::column(&*self.inner, index)
    }

    fn rows(&self) -> Rc<dyn IRows> {
        ITreeDataGridSource::rows(&*self.inner)
    }

    fn selection(&self) -> Option<Rc<dyn ITreeDataGridSelection>> {
        ITreeDataGridSource::selection(&*self.inner)
    }

    fn set_selection(&self, value: Option<Rc<dyn ITreeDataGridSelection>>) {
        ITreeDataGridSource::set_selection(&*self.inner, value);
    }

    fn is_hierarchical(&self) -> bool {
        false
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
        self.sorted.add(handler)
    }

    fn remove_sorted(&self, token: u64) {
        self.sorted.remove(token);
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

impl ITreeDataGridSourceOf<TestRow> for TrackingSource {
    fn items(&self) -> Models<TestRow> {
        ITreeDataGridSourceOf::items(&*self.inner)
    }

    fn set_items(&self, value: Models<TestRow>) {
        ITreeDataGridSourceOf::set_items(&*self.inner, value);
    }

    fn columns(&self) -> &ColumnList<TestRow> {
        ITreeDataGridSourceOf::columns(&*self.inner)
    }

    fn get_model_children(&self, model: &Rc<TestRow>) -> Option<Models<TestRow>> {
        ITreeDataGridSourceOf::get_model_children(&*self.inner, model)
    }
}

/// `AssertSubscriptionCounts`: the reference counts two `LayoutInvalidated` subscribers of the
/// source's columns while attached (the presenters of the grid); the port counts the grid's one
/// subscription to the changes of the source's core column (see [`TrackingSource`]).
fn assert_subscription_counts(source: &TrackingSource, source_event_count: usize, column_event_count: usize) {
    assert_eq!(source_event_count, source.property_changed_subscriber_count());
    assert_eq!(source_event_count, source.sorted_subscriber_count());
    assert_eq!(column_event_count, source.column_subscriber_count());
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// `new Binding(path)`.
fn binding(path: &str) -> Option<Rc<dyn BindingBase>> {
    Some(ReflectionBinding::new(path) as Rc<dyn BindingBase>)
}

/// The notifying list of the items of a test (the reference's notifying list of rows), as untyped
/// values.
fn items<T: PartialEq + 'static>(rows: &[Rc<T>]) -> Rc<FerroList<BoxedValue>> {
    let list = FerroList::new();
    for row in rows {
        list.add(row.clone() as BoxedValue);
    }
    Rc::new(list)
}

fn items_source(items: &Rc<FerroList<BoxedValue>>) -> Option<ItemsSource> {
    Some(ItemsSource::from(items.clone()))
}

/// A declarative text column with a header and a binding.
fn text_column(header_text: Option<&str>, binding: Option<Rc<dyn BindingBase>>) -> Rc<TreeDataGridTextColumn> {
    let column = TreeDataGridTextColumn::new();
    if let Some(header_text) = header_text {
        column.set_header_text(header_text);
    }
    column.set_binding(binding);
    column
}

/// A declarative check box column with a header and a binding.
fn check_box_column(header_text: &str, binding: Option<Rc<dyn BindingBase>>) -> Rc<TreeDataGridCheckBoxColumn> {
    let column = TreeDataGridCheckBoxColumn::new();
    column.set_header_text(header_text);
    column.set_binding(binding);
    column
}

/// The declarative expander column of the hierarchical tests: the children, whether a row has
/// children and whether it is expanded bound to the members of [`TestRow`], over a text column of
/// the name.
fn name_expander_column() -> Rc<TreeDataGridHierarchicalExpanderColumn> {
    let column = TreeDataGridHierarchicalExpanderColumn::new();
    column.set_header_text("Name");
    column.set_children_binding(binding("Children"));
    column.set_has_children_binding(binding("HasChildren"));
    column.set_is_expanded_binding(binding("IsExpanded"));
    column.set_inner_column(Some(text_column(None, binding("Name")) as Rc<dyn TreeDataGridColumn>));
    column
}

/// `CreateTarget()`.
fn create_target() -> Ref<TreeDataGrid> {
    tree_data_grid_with_template()
}

/// `CreateWindow(target)`: a window of 100 x 100 showing `target`, whose rows have the test
/// template and whose cells are 24 pixels high.
fn create_window(target: &Ref<TreeDataGrid>) -> Ref<Window> {
    let window = Window::new();
    window.styles().add(tree_data_grid_row_style());
    window.styles().add(Style::with_setters(
        Selectors::is::<TreeDataGridCell>(),
        [Setter::new(Layoutable::height_property(), 24.0)],
    ));
    window.set_width(100.0);
    window.set_height(100.0);
    window.show();
    // Running the jobs applies the template and updates the client size.
    run_jobs();
    window.set_content(Some(Control::boxed(target.clone())));
    window
}

/// `root.UpdateLayout(); Dispatcher.UIThread.RunJobs();`.
fn layout_and_run_jobs(root: &Window) {
    root.update_layout();
    run_jobs();
}

fn rows_of(target: &TreeDataGrid) -> Rc<dyn ITreeDataGridRows> {
    target.rows().expect("the rows of the grid")
}

fn columns_of(target: &TreeDataGrid) -> Rc<dyn IColumns> {
    target.columns().expect("the columns of the grid")
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[test]
fn items_source_columns_create_flat_source_and_rows() {
    let _app = start_app();
    let one = TestRow::named("One");
    one.age.set(10);
    one.is_active.set(true);
    let two = TestRow::named("Two");
    two.age.set(20);
    two.is_active.set(false);
    let items = items(&[one, two]);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(text_column(Some("Name"), binding("Name")));
    target.column_definitions().add_column(text_column(Some("Age"), binding("Age")));
    target.column_definitions().add_column(check_box_column("Active", binding("IsActive")));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let source = target.source().expect("a generated source");
    assert!(!source.is_hierarchical());
    assert_eq!(3, columns_of(&target).count());
    assert_eq!(2, rows_of(&target).count());

    let rows_presenter = target.rows_presenter().expect("the rows presenter");
    let rows = visual_children_of::<TreeDataGridRow>(&rows_presenter);

    assert_eq!(2, rows.len());
    let cells_presenter = rows[0].cells_presenter().expect("the cells presenter of the row");
    assert_eq!(3, visual_children_of::<Control>(&cells_presenter).len());

    root.close();
}

#[test]
fn declarative_items_source_preserves_collection_tracking() {
    let _app = start_app();
    let items = items(&[TestRow::named("One")]);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(text_column(Some("Name"), binding("Name")));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    assert_eq!(1, rows_of(&target).count());

    items.add(TestRow::named("Two") as BoxedValue);
    layout_and_run_jobs(&root);

    assert_eq!(2, rows_of(&target).count());

    items.remove_at(0);
    layout_and_run_jobs(&root);

    assert_eq!(1, rows_of(&target).count());

    root.close();
}

#[test]
fn items_source_hierarchical_columns_create_hierarchical_source() {
    let _app = start_app();
    let root_row = TestRow::named("Root");
    root_row.age.set(50);
    root_row.is_expanded.set(true);
    let child = TestRow::named("Child");
    child.age.set(10);
    root_row.add_child(&child);
    let items = items(&[root_row]);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(name_expander_column());
    target.column_definitions().add_column(text_column(Some("Age"), binding("Age")));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let source = target.source().expect("a generated source");
    assert!(source.is_hierarchical());
    assert_eq!(2, rows_of(&target).count());

    root.close();
}

#[test]
fn selection_mode_cell_multiple_raises_control_selection_changed() {
    let _app = start_app();
    let one = TestRow::named("One");
    one.age.set(10);
    let two = TestRow::named("Two");
    two.age.set(20);
    let items = items(&[one, two]);

    let target = create_target();
    target.set_selection_mode(TreeDataGridSelectionMode::CELL | TreeDataGridSelectionMode::MULTIPLE);
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(text_column(Some("Name"), binding("Name")));
    target.column_definitions().add_column(text_column(Some("Age"), binding("Age")));

    let raised: Rc<RefCell<Option<TreeDataGridSelectionChangedEventArgs>>> = Rc::new(RefCell::new(None));
    let sink = raised.clone();
    let _subscription = target.selection_changed(move |e: &TreeDataGridSelectionChangedEventArgs| {
        *sink.borrow_mut() = Some(e.clone());
    });

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let selection = target.column_selection().expect("a cell selection");
    selection.set_selected_range(&CellIndex::new(0, IndexPath::new(0)), 2, 2);

    let raised = raised.borrow().clone().expect("a raised selection change");
    assert_eq!(4, raised.selected_cell_indexes().len());
    assert_eq!(2, raised.selected_items().len());

    root.close();
}

#[test]
fn selection_subscriptions_are_scoped_to_visual_attachment() {
    let _app = start_app();
    let items = items(&[TestRow::named("One"), TestRow::named("Two")]);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(text_column(None, binding("Name")));

    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();
    let _subscription = target.selection_changed(move |_: &TreeDataGridSelectionChangedEventArgs| {
        counter.set(counter.get() + 1);
    });

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let selection = target.row_selection().expect("a row selection");

    root.set_content(None);
    selection.select(&IndexPath::new(0));
    assert_eq!(0, raised.get());

    root.set_content(Some(Control::boxed(target.clone())));
    layout_and_run_jobs(&root);
    selection.select(&IndexPath::new(1));
    assert_eq!(1, raised.get());

    root.set_content(None);
    root.set_content(Some(Control::boxed(target.clone())));
    layout_and_run_jobs(&root);
    selection.select(&IndexPath::new(0));
    assert_eq!(2, raised.get());

    root.close();
}

#[test]
fn source_and_column_subscriptions_are_scoped_to_visual_attachment() {
    let _app = start_app();
    let source = TrackingSource::new(vec![TestRow::named("One"), TestRow::named("Two")]);
    let target = create_target();
    target.set_source(Some(TreeDataGridSourceRef::of::<TestRow, TrackingSource>(source.clone())));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    assert_subscription_counts(&source, 1, 1);

    for _ in 0..3 {
        root.set_content(None);
        layout_and_run_jobs(&root);

        assert_subscription_counts(&source, 0, 0);

        root.set_content(Some(Control::boxed(target.clone())));
        layout_and_run_jobs(&root);

        assert_subscription_counts(&source, 1, 1);
    }

    root.close();
}

#[test]
fn explicit_source_can_disable_selection() {
    let _app = start_app();
    let source = FlatTreeDataGridSource::new(observable(vec![TestRow::named("One")]) as Models<TestRow>);
    ITreeDataGridSource::set_selection(&*source, None);
    source.columns().add(TextColumn::new(
        header("Name"),
        Accessor::member("Name", |x: &Rc<TestRow>| x.name()),
        None,
        None,
    ) as CoreColumnRef<TestRow>);

    let target = create_target();
    target.set_source(Some(TreeDataGridSourceRef::from(source.clone())));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    assert!(ITreeDataGridSource::selection(&*source).is_none());
    assert!(target.row_selection().is_none());
    assert!(target.column_selection().is_none());

    root.close();
}

#[test]
fn declarative_text_column_writes_back_through_binding_accessor() {
    let _app = start_app();
    let row = TestRow::new();
    row.age.set(10);
    let items = items(&[row.clone()]);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(text_column(Some("Age"), binding("Age")));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let rows = rows_of(&target);
    let cell = rows.realize_cell(&columns_of(&target).get(0), 0, 0);
    let text_cell = cell.as_text_cell().expect("a text cell");

    text_cell.set_text(Some("42".to_string()));

    assert_eq!(42, row.age.get());
    rows.unrealize_cell(&cell, 0, 0);

    root.close();
}

#[test]
fn declarative_text_column_supports_compiled_binding() {
    let _app = start_app();
    let row = TestRow::new();
    row.age.set(10);
    let items = items(&[row.clone()]);

    // `CompiledBinding.Create((TestRow x) => x.Age)`: a compiled binding whose path is the `Age`
    // property of the row.
    let age = Rc::new(ClrPropertyInfo::read_write::<TestRow, Value<i32>>("Age", |o| o.age.get(), |o, v| o.age.set(v)));
    let path = CompiledBindingPathBuilder::new()
        .property(age, PropertyInfoAccessorFactory::create_plain_property_accessor())
        .build();
    let compiled: Rc<dyn BindingBase> = CompiledBinding::new(path);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(text_column(Some("Age"), Some(compiled)));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let rows = rows_of(&target);
    let cell = rows.realize_cell(&columns_of(&target).get(0), 0, 0);
    let text_cell = cell.as_text_cell().expect("a text cell");

    assert_eq!(Some("10".to_string()), text_cell.text());

    text_cell.set_text(Some("55".to_string()));

    assert_eq!(55, row.age.get());
    rows.unrealize_cell(&cell, 0, 0);

    root.close();
}

#[test]
fn declarative_text_column_tracks_nested_reflection_binding_notifications() {
    let _app = start_app();
    let row = notifying_row_in_warsaw();
    let items = items(&[row.clone()]);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(text_column(Some("City"), binding("Customer.Address.City")));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let rows = rows_of(&target);
    let cell = rows.realize_cell(&columns_of(&target).get(0), 0, 0);
    let text_cell = cell.as_text_cell().expect("a text cell");
    assert_eq!(Some("Warsaw".to_string()), text_cell.text());

    row.customer().address().set_city(Some("Krakow".to_string()));
    run_jobs();

    assert_eq!(Some("Krakow".to_string()), text_cell.text());
    rows.unrealize_cell(&cell, 0, 0);

    root.close();
}

#[test]
fn declarative_text_column_tracks_nested_compiled_binding_notifications() {
    let _app = start_app();
    let row = notifying_row_in_warsaw();
    let items = items(&[row.clone()]);

    // `CompiledBinding.Create((NotifyingTestRow x) => x.Customer.Address.City)`: the three
    // properties of the path, each on a type that notifies its changes.
    let path = CompiledBindingPathBuilder::new()
        .notifying_property::<NotifyingTestRow, ModelRef<CustomerInfo>>(
            "Customer",
            |o| Some(o.customer()),
            |o, v| {
                if let Some(v) = v {
                    o.set_customer(v);
                }
            },
        )
        .notifying_property::<CustomerInfo, ModelRef<AddressInfo>>(
            "Address",
            |o| Some(o.address()),
            |o, v| {
                if let Some(v) = v {
                    o.set_address(v);
                }
            },
        )
        .notifying_property::<AddressInfo, Maybe<String>>("City", |o| o.city(), |o, v| o.set_city(v))
        .build();
    let compiled: Rc<dyn BindingBase> = CompiledBinding::new(path);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(text_column(Some("City"), Some(compiled)));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let rows = rows_of(&target);
    let cell = rows.realize_cell(&columns_of(&target).get(0), 0, 0);
    let text_cell = cell.as_text_cell().expect("a text cell");
    assert_eq!(Some("Warsaw".to_string()), text_cell.text());

    row.customer().address().set_city(Some("Krakow".to_string()));
    run_jobs();

    assert_eq!(Some("Krakow".to_string()), text_cell.text());
    rows.unrealize_cell(&cell, 0, 0);

    root.close();
}

#[test]
fn declarative_check_box_column_writes_back_through_binding_accessor() {
    let _app = start_app();
    let row = TestRow::new();
    row.is_active.set(true);
    let items = items(&[row.clone()]);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(check_box_column("Active", binding("IsActive")));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let rows = rows_of(&target);
    let cell = rows.realize_cell(&columns_of(&target).get(0), 0, 0);
    let check_box_cell = cell.as_any().downcast_ref::<CheckBoxCell>().expect("a check box cell");

    check_box_cell.set_value(Some(false));

    assert!(!row.is_active.get());
    rows.unrealize_cell(&cell, 0, 0);

    root.close();
}

#[test]
fn declarative_check_box_column_uses_nullable_binding_type_for_three_state_when_source_is_empty() {
    let _app = start_app();
    // The reference passes the model type with the null sample (`modelType: typeof(TestRow)`)
    // and resolves the type of the reflection binding's member on it. The port resolves the
    // members of a reflection binding on the models at run time, so without a sample the type of
    // the value is the type a compiled binding's path declares: here `bool?`.
    let is_optional = Rc::new(ClrPropertyInfo::read_write::<TestRow, Maybe<bool>>(
        "IsOptional",
        |o| o.is_optional.get(),
        |o, v| o.is_optional.set(v),
    ));
    let path = CompiledBindingPathBuilder::new()
        .property(is_optional, PropertyInfoAccessorFactory::create_plain_property_accessor())
        .build();
    let column_definition = check_box_column("Optional", Some(CompiledBinding::new(path) as Rc<dyn BindingBase>));
    column_definition.initialize_from_sample(None);

    let column = column_definition.create_untyped_column();
    let model = TestRow::new();
    model.is_optional.set(Some(false));
    let row: RowRef = AnonymousRow::<BoxedValue>::of(0, Rc::new(model.clone() as BoxedValue));
    let cell = column.create_cell(&row);
    let check_box_cell = cell.as_any().downcast_ref::<CheckBoxCell>().expect("a check box cell");

    assert!(check_box_cell.is_three_state());

    check_box_cell.set_value(None);

    assert_eq!(None, model.is_optional.get());
    cell.dispose();
}

#[test]
fn hierarchical_is_expanded_binding_writes_back_on_expand_and_collapse() {
    let _app = start_app();
    let root_row = TestRow::named("Root");
    root_row.is_expanded.set(true);
    root_row.add_child(&TestRow::named("Child"));
    let items = items(&[root_row.clone()]);

    let target = create_target();
    target.set_items_source(items_source(&items));
    target.column_definitions().add_column(name_expander_column());

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let source = target
        .source()
        .expect("a generated source")
        .downcast::<HierarchicalTreeDataGridSource<BoxedValue>>()
        .expect("a hierarchical source of untyped models");

    source.collapse(&IndexPath::new(0));
    assert!(!root_row.is_expanded.get());

    source.expand(&IndexPath::new(0));
    assert!(root_row.is_expanded.get());

    root.close();
}

#[test]
fn hierarchical_expander_text_column_infers_setter_from_getter() {
    let _app = start_app();
    let root_row = TestRow::named("Root");
    root_row.add_child(&TestRow::named("Child"));

    // `WithHierarchicalExpanderTextColumn(x => x.Name, x => x.Children)`: the reference infers the
    // setter from the member expression; the port's accessor states it.
    let source = HierarchicalTreeDataGridSource::new(observable(vec![root_row.clone()]) as Models<TestRow>)
        .with_hierarchical_expander_text_column(
            None,
            Accessor::member("Name", |x: &Rc<TestRow>| x.name())
                .with_setter(|x: &Rc<TestRow>, value: Option<String>| x.set_name(value)),
            typed_children,
            |_| {},
        );

    let target = create_target();
    target.set_source(Some(TreeDataGridSourceRef::from(source.clone())));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let rows = rows_of(&target);
    let cell = rows.realize_cell(&columns_of(&target).get(0), 0, 0);
    let expander_cell: &dyn IExpanderCellPresentation = cell.as_expander_cell().expect("an expander cell");
    let content = expander_cell.content().expect("the content of the expander cell");
    let text_cell = content.as_text_cell().expect("a text cell");

    text_cell.set_text(Some("Updated".to_string()));

    assert_eq!(Some("Updated".to_string()), root_row.name());
    rows.unrealize_cell(&cell, 0, 0);

    root.close();
}

#[test]
fn with_text_column_is_read_only_does_not_infer_setter() {
    let _app = start_app();
    let row = TestRow::named("Original");

    // The accessor has a setter (what the reference would infer); a read-only column drops it.
    let source = FlatTreeDataGridSource::new(observable(vec![row.clone()]) as Models<TestRow>).with_text_column(
        None,
        Accessor::member("Name", |x: &Rc<TestRow>| x.name())
            .with_setter(|x: &Rc<TestRow>, value: Option<String>| x.set_name(value)),
        |options| options.is_read_only = true,
    );

    let first = source.columns().get(0);
    let column = CoreColumn::as_any(&*first)
        .downcast_ref::<TextColumn<TestRow, Option<String>>>()
        .expect("a text column of the name");
    assert!(column.value_base().accessor().setter().is_none());

    let target = create_target();
    target.set_source(Some(TreeDataGridSourceRef::from(source.clone())));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let rows = rows_of(&target);
    let cell = rows.realize_cell(&columns_of(&target).get(0), 0, 0);
    let text_cell = cell.as_text_cell().expect("a text cell");

    assert!(!text_cell.can_edit());

    text_cell.set_text(Some("Updated".to_string()));

    assert_eq!(Some("Original".to_string()), row.name());
    rows.unrealize_cell(&cell, 0, 0);

    root.close();
}

#[test]
fn with_three_state_check_box_column_is_read_only_does_not_infer_setter() {
    let _app = start_app();
    let row = TestRow::new();
    row.is_optional.set(Some(false));

    // The accessor has a setter (what the reference would infer); a read-only column drops it.
    let source = FlatTreeDataGridSource::new(observable(vec![row.clone()]) as Models<TestRow>)
        .with_three_state_check_box_column(
            None,
            Accessor::member("IsOptional", |x: &Rc<TestRow>| x.is_optional.get())
                .with_setter(|x: &Rc<TestRow>, value: Option<bool>| x.is_optional.set(value)),
            |options| options.is_read_only = true,
        );

    let first = source.columns().get(0);
    let column = CoreColumn::as_any(&*first)
        .downcast_ref::<CheckBoxColumn<TestRow>>()
        .expect("a check box column of the optional flag");
    assert!(column.value_base().accessor().setter().is_none());

    let target = create_target();
    target.set_source(Some(TreeDataGridSourceRef::from(source.clone())));

    let root = create_window(&target);
    layout_and_run_jobs(&root);

    let rows = rows_of(&target);
    let cell = rows.realize_cell(&columns_of(&target).get(0), 0, 0);
    let check_box_cell = cell.as_any().downcast_ref::<CheckBoxCell>().expect("a check box cell");

    assert!(check_box_cell.is_read_only());

    check_box_cell.set_value(Some(true));

    assert_eq!(Some(false), row.is_optional.get());
    rows.unrealize_cell(&cell, 0, 0);

    root.close();
}
