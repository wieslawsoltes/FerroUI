//! `CorePresentationRegressionTests.cs`: the regressions of the presentation of the core's
//! sources: nested expansion bindings, the disposal of custom view columns, failed presentation
//! keys, duplicate and value-equal core columns, and the observation of whether a row shows its
//! expander.
//!
//! The reference's "classic" sources and columns (`HierarchicalTreeDataGridSource<Node>` with the
//! control's `HierarchicalExpanderColumn<Node>`) are the core's sources holding the control
//! crate's view columns in the port; their rows realize cells through the presentation's rows
//! (`TreeDataGridRows`), which is what the classic rows did.

use super::support::{assert_panics, presentation_of, start_app, ListDebug, TestNotifier};
use crate::models::{
    CellRef, ColumnRef, HierarchicalExpanderColumn, ICellColumn, IColumn, ITreeDataGridRows, IUpdateColumnLayout,
    IViewColumn, ShowExpanderObservable, TextColumn,
};
use crate::presentation::{
    register_disposable_column, IDisposableColumn, TreeDataGridPresentation, TreeDataGridPresentationOptions,
    TreeDataGridRows, TreeDataGridSourcePresentation,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::{ferro_model, BoxedValue};
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::models::{
    self as core_models, ColumnKind, ColumnRef as CoreColumnRef, Comparison, IColumn as CoreColumn, IColumnOf,
    IUntypedValueColumn, ListSortDirection, RowRef, ValueColumn,
};
use ferroui_treedatagrid_core::{
    Accessor, FlatTreeDataGridSource, GridLength as CoreGridLength, HierarchicalTreeDataGridSource, ITreeDataGridSource,
    Models,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// Models and helpers
// ---------------------------------------------------------------------------------------------

/// `Node`: a constant name, an expansion state that notifies its changes, and the children.
struct Node {
    state: Rc<State>,
    children: Rc<FerroList<Rc<Node>>>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Node {
    fn new() -> Rc<Self> {
        Rc::new(Self { state: State::new(), children: Rc::new(FerroList::new()) })
    }

    fn name(&self) -> String {
        "Node".to_string()
    }
}

/// `State`: the expansion state of a node, which notifies its changes.
struct State {
    notifier: TestNotifier,
    expanded: Cell<bool>,
}

impl State {
    fn new() -> Rc<Self> {
        Model::new_model(Self { notifier: TestNotifier::new(), expanded: Cell::new(false) })
    }

    fn is_expanded(&self) -> bool {
        self.expanded.get()
    }

    fn set_is_expanded(&self, value: bool) {
        self.expanded.set(value);
        self.notifier.raise("IsExpanded");
    }
}

impl INotifyPropertyChanged for State {
    fn property_changed(&self) -> &Event<str> {
        self.notifier.event()
    }
}

ferro_model!(State, |b| b.notify_property_changed());

/// `ReplaceableNode`: a model whose child collection is replaced, with a notification.
struct ReplaceableNode {
    notifier: TestNotifier,
    children: RefCell<Rc<ListDebug<ReplaceableNode>>>,
}

impl ReplaceableNode {
    fn new() -> Rc<Self> {
        Model::new_model(Self { notifier: TestNotifier::new(), children: RefCell::new(ListDebug::new()) })
    }

    fn children(&self) -> Rc<ListDebug<ReplaceableNode>> {
        self.children.borrow().clone()
    }

    fn set_children(&self, value: Rc<ListDebug<ReplaceableNode>>) {
        *self.children.borrow_mut() = value;
        self.notifier.raise("Children");
    }
}

impl INotifyPropertyChanged for ReplaceableNode {
    fn property_changed(&self) -> &Event<str> {
        self.notifier.event()
    }
}

ferro_model!(ReplaceableNode, |b| b.notify_property_changed());

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

fn header_text(header: Option<BoxedValue>) -> Option<String> {
    header.and_then(|header| header.downcast_ref::<String>().cloned())
}

/// Whether two shared values are the same object (`Assert.Same`).
fn same<A: ?Sized, B: ?Sized>(a: &Rc<A>, b: &Rc<B>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// `x => x.Name`.
fn name_accessor() -> Accessor<Node, String> {
    Accessor::member("Name", |x: &Rc<Node>| x.name())
}

/// `x => x.State.IsExpanded`: the member `IsExpanded` of the node's state, observed below the
/// node, written back with the state's setter.
fn is_expanded_accessor() -> Accessor<Node, bool> {
    Accessor::member("IsExpanded", |x: &Rc<Node>| x.state.is_expanded())
        .through(|x: &Rc<Node>| Some(x.state.clone() as BoxedValue), "IsExpanded")
        .with_setter(|x: &Rc<Node>, value: bool| x.state.set_is_expanded(value))
}

fn children_of(x: &Rc<Node>) -> Option<Models<Node>> {
    Some(x.children.clone() as Models<Node>)
}

fn view_text_column(header_value: &str) -> ColumnRef {
    TextColumn::<Node, String>::new(header(header_value), name_accessor(), None, None) as ColumnRef
}

// ---------------------------------------------------------------------------------------------
// DisposableColumn
// ---------------------------------------------------------------------------------------------

/// `DisposableColumn`: a text column of the name that counts its disposals. The reference derives
/// from `TextColumn<Node, string>`; the port holds one and forwards the view contracts to it.
struct DisposableColumn {
    inner: ColumnRef,
    dispose_count: Cell<i32>,
}

impl DisposableColumn {
    fn new() -> Rc<Self> {
        Rc::new(Self { inner: view_text_column("Name"), dispose_count: Cell::new(0) })
    }

    fn dispose_count(&self) -> i32 {
        self.dispose_count.get()
    }

    fn layout(&self) -> &dyn IUpdateColumnLayout {
        self.inner.as_update_column_layout().expect("a text column lays itself out")
    }

    fn cells(&self) -> &dyn ICellColumn {
        self.inner.as_cell_column().expect("a text column creates cells")
    }
}

impl IDisposableColumn for DisposableColumn {
    fn dispose(&self) {
        self.dispose_count.set(self.dispose_count.get() + 1);
    }
}

impl INotifyPropertyChanged for DisposableColumn {
    fn property_changed(&self) -> &Event<str> {
        self.inner.property_changed()
    }
}

impl IColumn for DisposableColumn {
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

impl IUpdateColumnLayout for DisposableColumn {
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

impl ICellColumn for DisposableColumn {
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
// ValueEqualColumn
// ---------------------------------------------------------------------------------------------

/// `ValueEqualColumn`: a neutral text column of the name whose instances are all equal
/// (`Equals` returns true for any other `ValueEqualColumn`). The reference derives from the
/// core's `TextColumn<Node, string>`; the port holds one and forwards the core contracts to it.
struct ValueEqualColumn {
    inner: Rc<ValueColumn<Node, String>>,
}

impl ValueEqualColumn {
    fn new(header_value: &str) -> Rc<Self> {
        Rc::new(Self { inner: core_models::TextColumn::new(header(header_value), name_accessor(), None, None, None) })
    }
}

impl PartialEq for ValueEqualColumn {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl INotifyPropertyChanged for ValueEqualColumn {
    fn property_changed(&self) -> &Event<str> {
        INotifyPropertyChanged::property_changed(&*self.inner)
    }
}

impl CoreColumn for ValueEqualColumn {
    fn id(&self) -> String {
        CoreColumn::id(&*self.inner)
    }

    fn header(&self) -> Option<BoxedValue> {
        CoreColumn::header(&*self.inner)
    }

    fn width(&self) -> CoreGridLength {
        CoreColumn::width(&*self.inner)
    }

    fn set_width(&self, value: CoreGridLength) {
        CoreColumn::set_width(&*self.inner, value);
    }

    fn is_visible(&self) -> bool {
        CoreColumn::is_visible(&*self.inner)
    }

    fn set_is_visible(&self, value: bool) {
        CoreColumn::set_is_visible(&*self.inner, value);
    }

    fn presentation_key(&self) -> Option<String> {
        CoreColumn::presentation_key(&*self.inner)
    }

    fn set_presentation_key(&self, value: Option<String>) {
        CoreColumn::set_presentation_key(&*self.inner, value);
    }

    fn sort_direction(&self) -> Option<ListSortDirection> {
        CoreColumn::sort_direction(&*self.inner)
    }

    fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        CoreColumn::set_sort_direction(&*self.inner, value);
    }

    fn tag(&self) -> Option<BoxedValue> {
        CoreColumn::tag(&*self.inner)
    }

    fn set_tag(&self, value: Option<BoxedValue>) {
        CoreColumn::set_tag(&*self.inner, value);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn model_type(&self) -> TypeId {
        TypeId::of::<Node>()
    }

    fn untyped_value(&self) -> Option<&dyn IUntypedValueColumn> {
        CoreColumn::untyped_value(&*self.inner)
    }
}

impl IColumnOf<Node> for ValueEqualColumn {
    fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<Node>> {
        IColumnOf::get_comparison(&*self.inner, direction)
    }

    fn kind(&self) -> ColumnKind<'_, Node> {
        IColumnOf::kind(&*self.inner)
    }
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

fn nested_expansion_binding_observes_nested_property_changes(native: bool) {
    let _app = start_app();
    let item = Node::new();
    item.children.add(Node::new());
    if native {
        let source = HierarchicalTreeDataGridSource::new(Rc::new(vec![item.clone()]) as Models<Node>);
        source.columns().add(core_models::HierarchicalExpanderColumn::new(
            core_models::TextColumn::new(header("Name"), name_accessor(), None, None, None) as CoreColumnRef<Node>,
            children_of,
            None,
            Some(is_expanded_accessor()),
            None,
        ) as CoreColumnRef<Node>);
        let view = presentation_of::<Node, _>(&source);
        let cell = view.rows().realize_cell(&view.columns().get(0), 0, 0);
        item.state.set_is_expanded(true);
        assert_eq!(2, ITreeDataGridSource::rows(&*source).count());
        view.rows().unrealize_cell(&cell, 0, 0);
    } else {
        let source = HierarchicalTreeDataGridSource::new(Rc::new(vec![item.clone()]) as Models<Node>);
        let column = HierarchicalExpanderColumn::<Node>::new(
            TextColumn::<Node, String>::new(header("Name"), name_accessor(), None, None) as Rc<dyn IViewColumn<Node>>,
            children_of,
            None,
            Some(is_expanded_accessor()),
        );
        source.columns().add(column.clone() as CoreColumnRef<Node>);
        // `source.Rows.RealizeCell(source.Columns[0], 0, 0)`: the classic rows realized the cells
        // of the source's view columns; the rows of a presentation over the core's rows do.
        let rows = TreeDataGridRows::new(ITreeDataGridSource::rows(&*source));
        let view_column = column.clone() as ColumnRef;
        let cell = rows.realize_cell(&view_column, 0, 0);
        item.state.set_is_expanded(true);
        assert_eq!(2, ITreeDataGridSource::rows(&*source).count());
        rows.unrealize_cell(&cell, 0, 0);
    }
}

#[test]
fn nested_expansion_binding_observes_nested_property_changes_classic() {
    nested_expansion_binding_observes_nested_property_changes(false);
}

#[test]
fn nested_expansion_binding_observes_nested_property_changes_native() {
    nested_expansion_binding_observes_nested_property_changes(true);
}

fn hierarchical_presentation_disposes_owned_custom_column(action: &str) {
    let _app = start_app();
    register_disposable_column::<DisposableColumn>();
    let source = HierarchicalTreeDataGridSource::new(Rc::new(vec![Node::new()]) as Models<Node>);
    source.columns().add(core_models::HierarchicalExpanderColumn::new(
        core_models::TemplateColumn::new::<Node>(header("Name"), "custom", None, None, None) as CoreColumnRef<Node>,
        children_of,
        None,
        None,
        None,
    ) as CoreColumnRef<Node>);
    let cell_column = DisposableColumn::new();
    let options = TreeDataGridPresentationOptions::new();
    let custom = cell_column.clone();
    options.set_column("custom", move |_: &Rc<dyn CoreColumn>| custom.clone() as ColumnRef);
    let view = TreeDataGridSourcePresentation::of::<Node, _>(source.clone(), Some(options.clone()));
    options.set_column("replacement", |_: &Rc<dyn CoreColumn>| view_text_column("Name"));

    if action == "hide" {
        source.columns().get(0).set_is_visible(false);
        assert_eq!(0, cell_column.dispose_count());
        view.dispose();
    } else if action == "replace" {
        source.columns().get(0).set_presentation_key(Some("replacement".to_string()));
    } else {
        view.dispose();
    }
    assert_eq!(1, cell_column.dispose_count());
    view.dispose();
    assert_eq!(1, cell_column.dispose_count());
}

#[test]
fn hierarchical_presentation_disposes_owned_custom_column_dispose() {
    hierarchical_presentation_disposes_owned_custom_column("dispose");
}

#[test]
fn hierarchical_presentation_disposes_owned_custom_column_hide() {
    hierarchical_presentation_disposes_owned_custom_column("hide");
}

#[test]
fn hierarchical_presentation_disposes_owned_custom_column_replace() {
    hierarchical_presentation_disposes_owned_custom_column("replace");
}

#[test]
fn failed_presentation_key_change_keeps_the_previous_column_recoverable() {
    let _app = start_app();
    register_disposable_column::<DisposableColumn>();
    let source = FlatTreeDataGridSource::new(Rc::new(vec![Node::new()]) as Models<Node>);
    let model = core_models::TemplateColumn::new::<Node>(header("Name"), "custom", None, None, None);
    source.columns().add(model.clone() as CoreColumnRef<Node>);
    let previous = DisposableColumn::new();
    let options = TreeDataGridPresentationOptions::new();
    let custom = previous.clone();
    options.set_column("custom", move |_: &Rc<dyn CoreColumn>| custom.clone() as ColumnRef);
    options.set_column("replacement", |_: &Rc<dyn CoreColumn>| view_text_column("Name"));
    let view = TreeDataGridSourcePresentation::of::<Node, _>(source.clone(), Some(options));
    let previous_view = view.columns().get(0);

    assert_panics(|| model.set_presentation_key(Some("missing".to_string())));
    assert!(same(&previous_view, &view.columns().get(0)));
    assert_eq!(0, previous.dispose_count());

    model.set_presentation_key(Some("custom".to_string()));
    assert!(same(&previous_view, &view.columns().get(0)));
    model.set_presentation_key(Some("replacement".to_string()));
    assert!(!same(&previous_view, &view.columns().get(0)));
    assert_eq!(1, previous.dispose_count());
}

#[test]
fn newly_added_failed_column_recovers_when_its_key_is_corrected() {
    let _app = start_app();
    let source = FlatTreeDataGridSource::new(Rc::new(vec![Node::new()]) as Models<Node>);
    source.columns().add(core_models::TextColumn::new(header("Name"), name_accessor(), None, None, None) as CoreColumnRef<Node>);
    let options = TreeDataGridPresentationOptions::new();
    options.set_column("recovered", |_: &Rc<dyn CoreColumn>| view_text_column("Recovered"));
    let view = TreeDataGridSourcePresentation::of::<Node, _>(source.clone(), Some(options));
    let failed = core_models::TemplateColumn::new::<Node>(header("Failed"), "missing", None, None, None);

    assert_panics(|| source.columns().add(failed.clone() as CoreColumnRef<Node>));
    assert_eq!(2, source.columns().count());
    assert_eq!(1, view.columns().count());

    failed.set_presentation_key(Some("recovered".to_string()));

    assert_eq!(2, view.columns().count());
    assert_eq!(Some("Recovered".to_string()), header_text(view.columns().get(1).header()));
}

#[test]
fn duplicate_core_column_is_rejected_before_the_presentation_changes() {
    let _app = start_app();
    let source = FlatTreeDataGridSource::new(Rc::new(vec![Node::new()]) as Models<Node>);
    let model = core_models::TextColumn::new(header("Name"), name_accessor(), None, None, None);
    source.columns().add(model.clone() as CoreColumnRef<Node>);
    let view = presentation_of::<Node, _>(&source);
    let cell = view.columns().get(0);

    assert_panics(|| source.columns().add(model.clone() as CoreColumnRef<Node>));

    assert_eq!(1, source.columns().count());
    assert!(same(&model, &source.columns().get(0)));
    assert_eq!(1, view.columns().count());
    assert!(same(&cell, &view.columns().get(0)));
}

#[test]
fn distinct_value_equal_core_columns_have_distinct_presentations() {
    let _app = start_app();
    let source = FlatTreeDataGridSource::new(Rc::new(vec![Node::new()]) as Models<Node>);
    let first = ValueEqualColumn::new("First");
    let second = ValueEqualColumn::new("Second");
    // The two columns are equal by value, as the reference's `Equals` override makes them.
    assert!(*first == *second);
    source.columns().add(first.clone() as CoreColumnRef<Node>);
    let view = presentation_of::<Node, _>(&source);

    source.columns().add(second.clone() as CoreColumnRef<Node>);

    assert_eq!(2, view.columns().count());
    source.columns().remove(&(first.clone() as CoreColumnRef<Node>));
    assert_eq!(1, source.columns().count());
    assert!(same(&second, &source.columns().get(0)));
    assert_eq!(1, view.columns().count());
    assert_eq!(Some("Second".to_string()), header_text(view.columns().get(0).header()));
}

#[test]
fn show_expander_observable_detaches_from_children_when_unsubscribed() {
    let _app = start_app();
    let children = ListDebug::<Node>::new();
    let observed = children.clone();
    let observable =
        ShowExpanderObservable::<Node>::new(Rc::new(move |_: &Rc<Node>| Some(observed.models())), None, Node::new());
    // `Subscribe(new BooleanObserver())` returns the subscription whose disposal is the
    // observable's `unsubscribe`.
    observable.subscribe(Rc::new(|_: bool| {}));

    assert_eq!(1, children.collection_changed_subscriber_count());

    observable.unsubscribe();

    assert_eq!(0, children.collection_changed_subscriber_count());
}

#[test]
fn show_expander_observable_rebinds_when_the_child_collection_is_replaced() {
    let _app = start_app();
    let old_children = ListDebug::<ReplaceableNode>::new();
    let new_children = ListDebug::<ReplaceableNode>::from_items([ReplaceableNode::new()]);
    let model = ReplaceableNode::new();
    model.set_children(old_children.clone());
    let observable = ShowExpanderObservable::<ReplaceableNode>::new(
        Rc::new(|x: &Rc<ReplaceableNode>| Some(x.children().models())),
        None,
        model.clone(),
    );
    // `BooleanObserver`: the last value pushed.
    let observer = Rc::new(Cell::new(false));
    let value = observer.clone();
    observable.subscribe(Rc::new(move |next: bool| value.set(next)));

    assert!(!observer.get());
    assert_eq!(1, old_children.collection_changed_subscriber_count());

    model.set_children(new_children.clone());

    assert!(observer.get());
    assert_eq!(0, old_children.collection_changed_subscriber_count());
    assert_eq!(1, new_children.collection_changed_subscriber_count());

    observable.unsubscribe();

    assert_eq!(0, new_children.collection_changed_subscriber_count());
}

#[test]
fn unrealized_core_expander_recomputes_show_expander_without_a_cached_override() {
    let _app = start_app();
    let item = Node::new();
    item.children.add(Node::new());
    let source = HierarchicalTreeDataGridSource::new(Rc::new(vec![item.clone()]) as Models<Node>);
    source.columns().add(core_models::HierarchicalExpanderColumn::new(
        core_models::TextColumn::new(header("Name"), name_accessor(), None, None, None) as CoreColumnRef<Node>,
        children_of,
        None,
        None,
        None,
    ) as CoreColumnRef<Node>);
    let view = presentation_of::<Node, _>(&source);
    let cell = view.rows().realize_cell(&view.columns().get(0), 0, 0);
    let row = ITreeDataGridSource::rows(&*source).get(0);
    assert!(row.as_expander().expect("an expander row").show_expander());

    view.rows().unrealize_cell(&cell, 0, 0);
    item.children.clear();

    let row = ITreeDataGridSource::rows(&*source).get(0);
    assert!(!row.as_expander().expect("an expander row").show_expander());
}
