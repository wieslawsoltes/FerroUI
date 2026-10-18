//! `TreeDataGridRowsPresenterTests.cs`.

use super::support::{
    column_list, empty_rows, has_logical_child, has_visual_child, is_red, layout, logical_children_of, red_brush,
    rows_of, run_jobs, same_model, scroll_viewer, start_app, test_window, visual_children_of, window_content,
    TreeAttachmentCounter,
};
use crate::models::{ColumnRef, IColumns, ITreeDataGridRows, TextColumn};
use crate::primitives::{TreeDataGridElementFactory, TreeDataGridRow, TreeDataGridRowsPresenter};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingOperations, ReflectionBinding, TemplateBinding};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_model, instantiate, BoxedValue, FerroObjectImpl, Ref, Size, StyledElementImpl, Vector, Visual,
    VisualImpl,
};
use ferroui_controls::presenters::{ContentPresenter, ItemsPresenter};
use ferroui_controls::primitives::{HeaderedContentControl, TemplatedControl};
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::{
    register_scroll_anchor_provider, Border, Control, ControlImpl, GridLength, IScrollAnchorProvider, ScrollViewer,
    StackPanel, TabControl, TabItem, Window,
};
use ferroui_treedatagrid_core::Accessor;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------------------------

struct RowModel {
    id: Cell<i32>,
    title: RefCell<Option<String>>,
    is_row_visible: Cell<bool>,
    property_changed: Event<str>,
}

ferro_model!(RowModel, |b| b
    .notify_property_changed()
    .read_only::<Value<i32>>("Id", |m| m.id.get())
    .property::<Value<bool>>("IsRowVisible", |m| m.is_row_visible.get(), |m, v| m.set_is_row_visible(v)));

impl INotifyPropertyChanged for RowModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl RowModel {
    fn new(id: i32, title: &str) -> Rc<Self> {
        Model::new_model(Self {
            id: Cell::new(id),
            title: RefCell::new(Some(title.to_string())),
            is_row_visible: Cell::new(true),
            property_changed: Event::new(),
        })
    }

    fn title(&self) -> String {
        self.title.borrow().clone().unwrap_or_default()
    }

    fn set_is_row_visible(&self, value: bool) {
        if self.is_row_visible.get() != value {
            self.is_row_visible.set(value);
            self.property_changed.raise("IsRowVisible");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Test classes
// ---------------------------------------------------------------------------------------------

/// `TestAnchorProvider`: a border that records the anchor candidates registered with it.
#[repr(C)]
struct TestAnchorProvider {
    base: Border,
    candidates: RefCell<Vec<Ref<Control>>>,
    unregistered: RefCell<Vec<Ref<Control>>>,
}

ferro_class!(TestAnchorProvider: Border);
ferroui_base::ferro_impl_classes!(
    TestAnchorProvider: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl IScrollAnchorProvider for TestAnchorProvider {
    fn current_anchor(&self) -> Option<Ref<Control>> {
        None
    }

    fn register_anchor_candidate(&self, element: &Ref<Control>) {
        assert!(self.is_visual_ancestor_of(element));
        let mut candidates = self.candidates.borrow_mut();
        if !candidates.contains(element) {
            candidates.push(element.clone());
        }
    }

    fn unregister_anchor_candidate(&self, element: &Ref<Control>) {
        self.candidates.borrow_mut().retain(|candidate| candidate != element);
        let mut unregistered = self.unregistered.borrow_mut();
        if !unregistered.contains(element) {
            unregistered.push(element.clone());
        }
    }
}

impl TestAnchorProvider {
    fn static_constructor() {
        register_scroll_anchor_provider::<TestAnchorProvider>();
    }

    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Border::construct(),
            candidates: RefCell::new(Vec::new()),
            unregistered: RefCell::new(Vec::new()),
        })
    }
}

/// `CountingRowsPresenter`: counts the measures of the presenter.
#[repr(C)]
struct CountingRowsPresenter {
    base: TreeDataGridRowsPresenter,
    measure_count: Cell<i32>,
}

ferro_class!(CountingRowsPresenter: TreeDataGridRowsPresenter);
ferroui_base::ferro_impl_classes!(
    CountingRowsPresenter: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for CountingRowsPresenter {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measure_count.set(this.measure_count.get() + 1);
        Self::parent_measure_override(this, available_size)
    }
}

impl CountingRowsPresenter {
    fn new() -> Ref<Self> {
        instantiate(Self { base: TreeDataGridRowsPresenter::construct(), measure_count: Cell::new(0) })
    }
}

// ---------------------------------------------------------------------------------------------
// Target
// ---------------------------------------------------------------------------------------------

struct Target {
    target: Ref<TreeDataGridRowsPresenter>,
    scroll: Ref<ScrollViewer>,
    items: Rc<FerroList<Rc<RowModel>>>,
    root: Ref<Window>,
}

struct Options {
    columns: Option<Rc<dyn IColumns>>,
    additional_styles: Vec<Ref<Style>>,
    item_count: usize,
    root_size: Option<Size>,
    cache_length: f64,
    presenter: Option<Ref<TreeDataGridRowsPresenter>>,
}

impl Default for Options {
    fn default() -> Self {
        Self { columns: None, additional_styles: Vec::new(), item_count: 100, root_size: None, cache_length: 0.0, presenter: None }
    }
}

fn create_target_with(options: Options) -> Target {
    let items = Rc::new(FerroList::from_items(
        (0..options.item_count).map(|x| RowModel::new(x as i32, &format!("Item {x}"))),
    ));

    let rows = rows_of(items.clone());

    let target = options.presenter.unwrap_or_else(TreeDataGridRowsPresenter::new);
    target.set_cache_length(options.cache_length);
    target.set_element_factory(Some(TreeDataGridElementFactory::new()));
    target.set_items(Some(rows as Rc<dyn ITreeDataGridRows>));
    target.set_columns(options.columns);

    let scroll = scroll_viewer(target.clone().upcast());

    let root = test_window(Some(scroll.clone().upcast()), options.root_size);
    root.styles().add(Style::with_setters(
        Selectors::of_type::<TreeDataGridRow>(),
        [Setter::new(Layoutable::height_property(), 10.0)],
    ));

    for style in options.additional_styles {
        root.styles().add(style);
    }

    root.update_layout();
    run_jobs();

    Target { target, scroll, items, root }
}

fn create_target() -> Target {
    create_target_with(Options::default())
}

fn realized_rows(target: &TreeDataGridRowsPresenter) -> Vec<Ref<TreeDataGridRow>> {
    target.realized_elements().into_iter().flatten().map(|element| element.cast::<TreeDataGridRow>().expect("a row")).collect()
}

fn row_at(elements: &[Option<Ref<Control>>], index: usize) -> Ref<TreeDataGridRow> {
    elements[index].as_ref().and_then(|element| element.cast::<TreeDataGridRow>()).expect("a row")
}

fn row(element: Option<Ref<Control>>) -> Ref<TreeDataGridRow> {
    element.and_then(|element| element.cast::<TreeDataGridRow>()).expect("a row")
}

fn range(first: i32, count: i32) -> Vec<i32> {
    (first..first + count).collect()
}

fn assert_row_indexes(target: &TreeDataGridRowsPresenter, first_row_index: i32, row_count: i32) {
    let mut row_indexes: Vec<i32> =
        visual_children_of::<TreeDataGridRow>(target).iter().filter(|x| x.is_visible()).map(|x| x.row_index()).collect();
    row_indexes.sort();

    assert_eq!(range(first_row_index, row_count), row_indexes);

    let mut row_indexes: Vec<i32> =
        realized_rows(target).iter().filter(|x| x.is_visible()).map(|x| x.row_index()).collect();
    row_indexes.sort();

    assert_eq!(range(first_row_index, row_count), row_indexes);
}

fn assert_recyclable(target: &TreeDataGridRowsPresenter, count: usize) {
    let recyclable = logical_children_of::<TreeDataGridRow>(target).into_iter().filter(|x| !x.is_visible()).count();
    assert_eq!(count, recyclable);
}

fn get_realized_row_indexes(target: &TreeDataGridRowsPresenter) -> Vec<i32> {
    target
        .realized_elements()
        .iter()
        .map(|element| element.as_ref().and_then(|element| element.cast::<TreeDataGridRow>()).map_or(-1, |row| row.row_index()))
        .collect()
}

fn model_of_row(row: &TreeDataGridRow) -> Rc<RowModel> {
    super::support::model_of::<RowModel>(row.data_context())
}

fn assert_realized_rows_are_consistent(target: &TreeDataGridRowsPresenter, items: &FerroList<Rc<RowModel>>) {
    let rows = realized_rows(target);
    let mut indexes: Vec<i32> = rows.iter().map(|x| x.row_index()).collect();
    indexes.sort();
    indexes.dedup();
    assert_eq!(rows.len(), indexes.len());

    for row in &rows {
        assert!(row.row_index() >= 0 && (row.row_index() as usize) < items.count());
        assert!(same_model(&items.get(row.row_index() as usize), row.data_context().as_ref()));
        assert!(row.is_visible());
        assert!(has_visual_child(target, row));
        assert!(has_logical_child(target, row));
    }

    let mut ordered = rows.clone();
    ordered.sort_by_key(|x| x.row_index());
    let mut visible: Vec<Ref<TreeDataGridRow>> =
        visual_children_of::<TreeDataGridRow>(target).into_iter().filter(|x| x.is_visible()).collect();
    visible.sort_by_key(|x| x.row_index());
    assert_eq!(ordered, visible);
}

fn assert_data_contexts_are_items(rows: &[Ref<TreeDataGridRow>], items: &[Rc<RowModel>]) {
    assert_eq!(items.len(), rows.len());
    for (row, item) in rows.iter().zip(items) {
        assert!(same_model(item, row.data_context().as_ref()));
    }
}

fn first_items(items: &FerroList<Rc<RowModel>>, count: usize) -> Vec<Rc<RowModel>> {
    items.to_vec().into_iter().take(count).collect()
}

fn focus(row: &TreeDataGridRow) {
    row.set_focusable(true);
    row.focus();
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[test]
fn nth_child_handles_deletion_and_addition_correctly() {
    let _app = start_app();
    let style = Style::with_setters(
        Selectors::of_type::<TreeDataGridRowsPresenter>().descendant().of_type::<TreeDataGridRow>().nth_child(2, 0),
        [Setter::new(TemplatedControl::background_property(), red_brush())],
    );
    let t = create_target_with(Options { additional_styles: vec![style], ..Options::default() });

    layout(&t.target);

    let count_even_red_rows = || {
        visual_children_of::<TreeDataGridRow>(&t.target).into_iter().filter(|row| is_red(row.background())).count()
    };

    assert_eq!(5, count_even_red_rows());
    assert_eq!(100, t.items.count());

    t.items.remove_at(0);
    t.items.remove_at(0);

    assert_eq!(98, t.items.count());

    layout(&t.target);

    assert_eq!(5, count_even_red_rows());

    t.items.add(RowModel::new(101, "Item 101"));

    assert_eq!(99, t.items.count());

    layout(&t.target);

    assert_eq!(5, count_even_red_rows());
}

#[test]
fn creates_initial_rows() {
    let _app = start_app();
    let t = create_target();

    assert_eq!(Size::new(100.0, 1000.0), t.scroll.extent());
    assert_row_indexes(&t.target, 0, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn scrolls_down_one_row() {
    let _app = start_app();
    let t = create_target();

    t.scroll.set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    assert_row_indexes(&t.target, 1, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn full_row_scroll_reuses_controls_without_tree_reattachment() {
    let _app = start_app();
    let t = create_target();
    let rows = realized_rows(&t.target);
    let counter = TreeAttachmentCounter::new();

    for row in &rows {
        counter.watch(row);
    }

    t.scroll.set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    let after = realized_rows(&t.target);
    assert_eq!(rows.len(), after.len());
    assert!(rows.iter().all(|row| after.contains(row)));
    counter.assert_none();
    assert_row_indexes(&t.target, 1, 10);
}

#[test]
fn cache_length_buffers_rows_across_small_scrolls() {
    let _app = start_app();
    let t = create_target_with(Options { cache_length: 0.5, ..Options::default() });
    let initial_rows = t.target.realized_elements();

    assert_row_indexes(&t.target, 0, 20);

    t.scroll.set_offset(Vector::new(0.0, 40.0));
    layout(&t.target);

    assert_eq!(initial_rows, t.target.realized_elements());

    t.scroll.set_offset(Vector::new(0.0, 110.0));
    layout(&t.target);

    assert_row_indexes(&t.target, 6, 20);
}

#[test]
fn buffered_rows_remain_attached_when_realized_count_shrinks_and_grows() {
    let _app = start_app();
    let t = create_target_with(Options { cache_length: 0.5, ..Options::default() });
    let rows = realized_rows(&t.target);
    let counter = TreeAttachmentCounter::new();

    assert_eq!(20, rows.len());

    for row in &rows {
        counter.watch(row);
    }

    t.target.set_cache_length(0.0);
    t.target.update_layout();

    assert_eq!(10, t.target.realized_elements().len());
    assert_recyclable(&t.target, 10);

    t.target.set_cache_length(0.5);
    t.target.update_layout();

    let after = realized_rows(&t.target);
    assert_eq!(rows.len(), after.len());
    assert!(rows.iter().all(|row| after.contains(row)));
    counter.assert_none();
    assert_recyclable(&t.target, 0);
}

#[test]
fn viewport_shrink_and_grow_preserves_realized_coverage() {
    let _app = start_app();
    let t = create_target_with(Options { cache_length: 0.5, ..Options::default() });

    t.root.set_height(10.0);
    t.root.invalidate_measure();
    layout(&t.target);

    t.root.set_height(100.0);
    t.root.invalidate_measure();
    layout(&t.target);

    assert_realized_rows_are_consistent(&t.target, &t.items);
    let realized = t.target.realized_elements();
    assert!(row_at(&realized, 0).bounds().top() <= t.scroll.offset().y);
    assert!(row_at(&realized, realized.len() - 1).bounds().bottom() >= t.scroll.offset().y + t.scroll.viewport().height);
}

#[test]
fn registers_only_viewport_rows_with_nearest_anchor_provider() {
    let _app = start_app();
    let t = create_target_with(Options { cache_length: 0.5, ..Options::default() });
    t.scroll.set_content(None);
    t.scroll.update_layout();
    run_jobs();

    let provider = TestAnchorProvider::new();
    provider.set_child(&t.target);
    t.scroll.set_content(Some(Control::boxed(&provider)));
    layout(&t.target);

    let viewport = t.target.viewport();
    let expected: Vec<Ref<TreeDataGridRow>> = realized_rows(&t.target)
        .into_iter()
        .filter(|x| x.is_visible() && viewport.intersects(x.bounds()))
        .collect();

    assert!(!expected.is_empty());
    {
        let candidates = provider.candidates.borrow();
        assert_eq!(expected.len(), candidates.len());
        assert!(expected.iter().all(|row| candidates.iter().any(|candidate| candidate.ptr_eq(row))));
        assert!(t.target.realized_elements().len() > candidates.len());
    }

    let focused = expected[0].clone();
    focus(&focused);
    t.target.recycle_all_elements();

    assert!(provider.unregistered.borrow().iter().any(|element| element.ptr_eq(&focused)));
    assert!(!provider.candidates.borrow().iter().any(|element| element.ptr_eq(&focused)));
}

#[test]
fn small_scrolls_within_realized_rows_do_not_repeat_measure() {
    let _app = start_app();
    let presenter = CountingRowsPresenter::new();
    let t = create_target_with(Options { presenter: Some(presenter.clone().upcast()), ..Options::default() });

    t.scroll.set_offset(Vector::new(0.0, 1.0));
    layout(&t.target);
    let measure_count_after_realizing_trailing_row = presenter.measure_count.get();

    for offset in 2..=10 {
        t.scroll.set_offset(Vector::new(0.0, f64::from(offset)));
        layout(&t.target);
    }

    assert_eq!(measure_count_after_realizing_trailing_row, presenter.measure_count.get());
    assert_realized_rows_are_consistent(&t.target, &t.items);

    t.scroll.set_offset(Vector::new(0.0, 11.0));
    layout(&t.target);

    assert!(presenter.measure_count.get() > measure_count_after_realizing_trailing_row);
    assert_realized_rows_are_consistent(&t.target, &t.items);
}

#[test]
fn scrolls_down_more_than_a_page() {
    let _app = start_app();
    let t = create_target();

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    assert_row_indexes(&t.target, 20, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn scrolls_up_more_than_a_page() {
    let _app = start_app();
    let t = create_target();

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);

    t.scroll.set_offset(Vector::new(0.0, 0.0));
    layout(&t.target);

    assert_row_indexes(&t.target, 0, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn handles_inserted_row() {
    let _app = start_app();
    let t = create_target();

    assert_eq!(10, t.target.realized_elements().len());

    let preserved_prefix: Vec<Option<Ref<Control>>> = t.target.realized_elements().into_iter().take(2).collect();

    t.items.insert(2, RowModel::new(100, "New"));

    let indexes = get_realized_row_indexes(&t.target);

    // The inserted slot is unrealized, while existing rows retain their items at their shifted
    // indexes until the next measure fills the slot and trims the tail.
    let mut expected = vec![0, 1, -1];
    expected.extend(range(3, 8));
    assert_eq!(expected, indexes);
    layout(&t.target);

    let indexes = get_realized_row_indexes(&t.target);

    assert_eq!(range(0, 10), indexes);
    assert_eq!(preserved_prefix, t.target.realized_elements().into_iter().take(2).collect::<Vec<_>>());
}

#[test]
fn handles_removed_row() {
    let _app = start_app();
    let t = create_target();

    assert_eq!(10, t.target.realized_elements().len());

    t.items.remove_at(2);

    let indexes = get_realized_row_indexes(&t.target);

    assert_eq!(range(0, 9), indexes);
    layout(&t.target);

    let indexes = get_realized_row_indexes(&t.target);

    assert_eq!(range(0, 10), indexes);
    assert_data_contexts_are_items(&realized_rows(&t.target), &first_items(&t.items, 10));
}

#[test]
fn insert_preserves_realized_rows_whose_items_only_changed_index() {
    let _app = start_app();
    let t = create_target();
    let rows = realized_rows(&t.target);
    let shifted_item = rows[2].data_context();

    t.items.insert(2, RowModel::new(100, "New"));

    assert_eq!(Some(rows[2].clone().upcast::<Control>()), t.target.try_get_element(3));
    assert!(super::support::same_value(shifted_item.as_ref(), rows[2].data_context().as_ref()));
    assert_eq!(3, rows[2].row_index());

    layout(&t.target);

    assert_eq!(Some(rows[0].clone().upcast::<Control>()), t.target.try_get_element(0));
    assert_eq!(Some(rows[1].clone().upcast::<Control>()), t.target.try_get_element(1));
    for old_index in 2..9 {
        assert_eq!(Some(rows[old_index].clone().upcast::<Control>()), t.target.try_get_element(old_index as i32 + 1));
    }
    assert_realized_rows_are_consistent(&t.target, &t.items);
}

#[test]
fn remove_preserves_realized_rows_whose_items_only_changed_index() {
    let _app = start_app();
    let t = create_target();
    let rows = realized_rows(&t.target);
    let shifted_item = rows[3].data_context();

    t.items.remove_at(2);

    assert_eq!(Some(rows[3].clone().upcast::<Control>()), t.target.try_get_element(2));
    assert!(super::support::same_value(shifted_item.as_ref(), rows[3].data_context().as_ref()));
    assert_eq!(2, rows[3].row_index());

    layout(&t.target);

    assert_eq!(Some(rows[0].clone().upcast::<Control>()), t.target.try_get_element(0));
    assert_eq!(Some(rows[1].clone().upcast::<Control>()), t.target.try_get_element(1));
    for old_index in 3..10 {
        assert_eq!(Some(rows[old_index].clone().upcast::<Control>()), t.target.try_get_element(old_index as i32 - 1));
    }
    assert_realized_rows_are_consistent(&t.target, &t.items);
}

#[test]
fn collection_edit_remains_consistent_across_recycling_scrolls() {
    let _app = start_app();
    let t = create_target();

    t.items.insert(2, RowModel::new(-1, "Inserted"));
    layout(&t.target);
    assert_realized_rows_are_consistent(&t.target, &t.items);

    t.scroll.set_offset(Vector::new(0.0, 100.0));
    layout(&t.target);
    assert_realized_rows_are_consistent(&t.target, &t.items);

    t.items.remove_at(2);
    layout(&t.target);
    assert_realized_rows_are_consistent(&t.target, &t.items);

    t.scroll.set_offset(Vector::default());
    layout(&t.target);
    assert_realized_rows_are_consistent(&t.target, &t.items);
}

#[test]
fn collection_edit_preserves_focus_on_a_shifted_visible_row() {
    let _app = start_app();
    let t = create_target();
    let focused_row = row_at(&t.target.realized_elements(), 5);
    let focused_item = focused_row.data_context();

    focus(&focused_row);
    assert!(focused_row.is_keyboard_focus_within());

    t.items.insert(2, RowModel::new(-1, "Inserted"));
    layout(&t.target);

    assert_eq!(Some(focused_row.clone().upcast::<Control>()), t.target.try_get_element(6));
    assert!(super::support::same_value(focused_item.as_ref(), focused_row.data_context().as_ref()));
    assert!(focused_row.is_keyboard_focus_within());

    t.items.remove_at(2);
    layout(&t.target);

    assert_eq!(Some(focused_row.clone().upcast::<Control>()), t.target.try_get_element(5));
    assert!(super::support::same_value(focused_item.as_ref(), focused_row.data_context().as_ref()));
    assert!(focused_row.is_keyboard_focus_within());
}

fn rows_by_item(target: &TreeDataGridRowsPresenter) -> Vec<(Rc<RowModel>, Ref<TreeDataGridRow>)> {
    realized_rows(target).into_iter().map(|row| (model_of_row(&row), row)).collect()
}

#[test]
fn move_preserves_every_realized_row_that_remains_in_the_viewport() {
    let _app = start_app();
    let t = create_target();
    let rows_by_item = rows_by_item(&t.target);

    t.items.move_item(2, 7);
    layout(&t.target);

    assert_realized_rows_are_consistent(&t.target, &t.items);
    for item in first_items(&t.items, 10) {
        let (_, row) = rows_by_item.iter().find(|(model, _)| Rc::ptr_eq(model, &item)).expect("the row of the item");
        let index = t.items.index_of(&item).expect("the index of the item") as i32;
        assert_eq!(Some(row.clone().upcast::<Control>()), t.target.try_get_element(index));
    }
}

#[test]
fn move_crossing_viewport_boundary_preserves_rows_that_stay_visible() {
    let _app = start_app();
    let t = create_target();
    t.scroll.set_offset(Vector::new(0.0, 100.0));
    layout(&t.target);

    let rows_by_item = rows_by_item(&t.target);

    t.items.move_item(2, 15);
    layout(&t.target);

    assert_realized_rows_are_consistent(&t.target, &t.items);
    for (item, row) in &rows_by_item {
        let new_index = t.items.index_of(item).expect("the index of the item") as i32;
        if let Some(element) = t.target.try_get_element(new_index) {
            assert!(element.ptr_eq(row));
        }
    }

    t.scroll.set_offset(Vector::default());
    layout(&t.target);
    assert_realized_rows_are_consistent(&t.target, &t.items);
}

#[test]
fn move_preserves_focus_and_row_identity_for_visible_item() {
    let _app = start_app();
    let t = create_target();
    let focused_row = row_at(&t.target.realized_elements(), 5);
    let focused_item = focused_row.data_context();

    focus(&focused_row);
    assert!(focused_row.is_keyboard_focus_within());

    t.items.move_item(5, 8);
    layout(&t.target);

    assert!(same_model(&t.items.get(8), focused_item.as_ref()));
    assert_eq!(Some(focused_row.clone().upcast::<Control>()), t.target.try_get_element(8));
    assert!(focused_row.is_keyboard_focus_within());
    assert_realized_rows_are_consistent(&t.target, &t.items);
}

#[test]
fn handles_unrealized_rows_being_removed_from_end() {
    let _app = start_app();
    let t = create_target();

    assert_eq!(Size::new(100.0, 1000.0), t.scroll.extent());
    assert_row_indexes(&t.target, 0, 10);
    assert_recyclable(&t.target, 0);

    t.items.remove_range(90, 10);

    assert_row_indexes(&t.target, 0, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn handles_unrealized_rows_being_removed_from_start() {
    let _app = start_app();
    let t = create_target();

    assert_eq!(Size::new(100.0, 1000.0), t.scroll.extent());
    t.scroll.set_offset(Vector::new(0.0, 900.0));
    layout(&t.target);

    assert_row_indexes(&t.target, 90, 10);
    assert_recyclable(&t.target, 0);

    t.items.remove_range(0, 10);

    assert_row_indexes(&t.target, 80, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn realized_children_should_not_be_removed() {
    let _app = start_app();
    let t = create_target();

    assert_eq!(Some(100), t.target.items().map(|items| items.count()));
    assert_eq!(10, t.target.realized_elements().len());

    t.items.remove_range(7, 93);
    layout(&t.target);
    let children = t.target.get_visual_children();
    let realized = t.target.realized_elements();

    for (i, child) in children.iter().enumerate() {
        assert!(realized[i].as_ref().is_some_and(|element| element.ptr_eq(child)));
    }
}

#[test]
fn realized_children_are_added_to_the_logical_tree() {
    let _app = start_app();
    let t = create_target();

    assert!(!t.target.realized_elements().is_empty());
    for element in t.target.realized_elements().into_iter().flatten() {
        assert!(has_logical_child(&t.target, &element));
    }
}

#[test]
fn recycled_rows_are_removed_from_visual_and_logical_trees() {
    let _app = start_app();
    let t = create_target();

    assert_eq!(10, t.target.realized_elements().len());

    t.target.recycle_all_elements();

    assert!(t.target.realized_elements().is_empty());
    assert!(t.target.get_visual_children().is_empty());
    assert!(t.target.get_logical_children().is_empty());
}

#[test]
fn detached_collection_shrink_does_not_leave_ghost_rows() {
    let _app = start_app();
    let t = create_target_with(Options { item_count: 10, ..Options::default() });
    let window = t.root.clone();
    assert!(window_content(&window).is_some_and(|content| content.ptr_eq(&t.scroll)));

    assert_eq!(10, t.target.get_visual_children().len());

    window.set_content(None);
    window.update_layout();
    t.items.remove_range(5, 5);
    window.set_content(Some(Control::boxed(&t.scroll)));
    window.update_layout();
    run_jobs();

    let visual_rows = visual_children_of::<TreeDataGridRow>(&t.target);
    let logical_rows = logical_children_of::<TreeDataGridRow>(&t.target);

    assert_eq!(5, visual_rows.len());
    assert_eq!(5, logical_rows.len());
    assert!(visual_rows.iter().all(|x| x.is_visible()));
    assert_data_contexts_are_items(&visual_rows, &t.items.to_vec());
    assert_eq!(visual_rows, logical_rows);
}

#[test]
fn should_remove_children_on_empty_collection_assignment_to_items() {
    let _app = start_app();
    let t = create_target();
    layout(&t.target);
    assert_eq!(100, t.items.count());
    t.items.remove_range(1, 99);
    layout(&t.target);
    assert!(t.target.items().is_some());
    assert_eq!(Some(1), t.target.items().map(|items| items.count()));
    assert_eq!(1, t.target.get_visual_children().len());

    t.target.set_items(Some(empty_rows::<RowModel>() as Rc<dyn ITreeDataGridRows>));
    layout(&t.target);
    assert_eq!(Some(0), t.target.items().map(|items| items.count()));

    assert!(t.target.get_visual_children().is_empty());
    assert!(t.target.get_logical_children().is_empty());

    let items = Rc::new(FerroList::from_items((0..5).map(|x| RowModel::new(x, &format!("Item {x}")))));
    t.target.set_items(Some(rows_of(items) as Rc<dyn ITreeDataGridRows>));
    layout(&t.target);
    assert_eq!(Some(5), t.target.items().map(|items| items.count()));

    assert_eq!(5, t.target.get_visual_children().len());
}

#[test]
fn handles_removed_and_reinserted_row() {
    let _app = start_app();
    let t = create_target();

    assert_eq!(10, t.target.realized_elements().len());

    let item = t.items.get(0);
    let preserved_rows: Vec<Option<Ref<Control>>> = t.target.realized_elements().into_iter().skip(1).collect();
    t.items.remove_at(0);

    let indexes = get_realized_row_indexes(&t.target);

    assert_eq!(range(0, 9), indexes);
    assert_eq!(preserved_rows, t.target.realized_elements());

    t.items.insert(0, item.clone());

    let indexes = get_realized_row_indexes(&t.target);
    let mut expected = vec![-1];
    expected.extend(range(1, 9));
    assert_eq!(expected, indexes);
    layout(&t.target);

    let indexes = get_realized_row_indexes(&t.target);

    assert_eq!(range(0, 10), indexes);
    assert!(same_model(&item, row_at(&t.target.realized_elements(), 0).data_context().as_ref()));
    assert_eq!(preserved_rows, t.target.realized_elements().into_iter().skip(1).collect::<Vec<_>>());
}

#[test]
fn handles_removing_row_range_that_spans_realized_and_unrealized_elements() {
    let _app = start_app();
    let t = create_target();

    // Scroll down one item.
    t.scroll.set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    assert_eq!(10, t.target.realized_elements().len());

    let to_recycle: Vec<Option<Ref<Control>>> = t.target.realized_elements().into_iter().skip(4).take(6).collect();
    t.items.remove_range(5, 10);

    let indexes = get_realized_row_indexes(&t.target);

    // Item removed from realized elements and subsequent row indexes updated.
    assert_eq!(range(1, 4), indexes);

    let mut elements = t.target.realized_elements();
    layout(&t.target);

    let indexes = get_realized_row_indexes(&t.target);

    // After layout an element for the newly visible last row is created and indexes updated.
    assert_eq!(range(1, 10), indexes);

    // And the removed row should now have been recycled as the last row.
    elements.extend(to_recycle);
    assert_eq!(elements, t.target.realized_elements());
}

#[test]
fn handles_removing_all_rows_when_scrolled() {
    let _app = start_app();
    let t = create_target();

    // Scroll down one item.
    t.scroll.set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    assert_eq!(10, t.target.realized_elements().len());

    // Remove all items using RemoveRange.
    t.items.remove_range(0, t.items.count());

    // All items removed.
    assert!(t.target.realized_elements().is_empty());
}

#[test]
fn handles_removing_row_range_that_invalidates_current_viewport() {
    let _app = start_app();
    let t = create_target();

    // Scroll down ten items.
    t.scroll.set_offset(Vector::new(0.0, 100.0));
    layout(&t.target);

    assert_eq!(10, t.target.realized_elements().len());

    // Remove all but the first five items.
    t.items.remove_range(5, 95);

    layout(&t.target);

    // The target bounds should be updated, which will cause the scroll viewer to scroll back up.
    assert_eq!(Size::new(100.0, 100.0), t.target.bounds().size());
}

#[test]
fn handles_removing_focused_row_while_outside_viewport() {
    let _app = start_app();
    let t = create_target();
    let element = row_at(&t.target.realized_elements(), 0);

    focus(&element);

    // Scroll down one item.
    t.scroll.set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    // Remove the focused element.
    t.items.remove_at(0);

    // Scroll back to the beginning.
    t.scroll.set_offset(Vector::new(0.0, 0.0));
    layout(&t.target);

    // The correct element should be shown.
    assert!(same_model(&t.items.get(0), row_at(&t.target.realized_elements(), 0).data_context().as_ref()));
}

#[test]
fn handles_replacing_focused_row_while_outside_viewport() {
    let _app = start_app();
    let t = create_target();
    let element = row_at(&t.target.realized_elements(), 0);

    focus(&element);

    // Scroll down one item.
    t.scroll.set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    // Replace the focused element.
    t.items.set(0, RowModel::new(100, "New Item"));

    // Scroll back to the beginning.
    t.scroll.set_offset(Vector::new(0.0, 0.0));
    layout(&t.target);

    // The correct element should be shown.
    assert!(same_model(&t.items.get(0), row_at(&t.target.realized_elements(), 0).data_context().as_ref()));
}

#[test]
fn handles_moving_focused_row_while_outside_viewport() {
    let _app = start_app();
    let t = create_target();
    let element = row_at(&t.target.realized_elements(), 0);

    focus(&element);

    // Scroll down one item.
    t.scroll.set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    // Move the focused element.
    t.items.move_item(0, t.items.count() - 1);

    // Scroll back to the beginning.
    t.scroll.set_offset(Vector::new(0.0, 0.0));
    layout(&t.target);

    // The correct element should be shown.
    assert!(same_model(&t.items.get(0), row_at(&t.target.realized_elements(), 0).data_context().as_ref()));
}

fn assert_offscreen_focused_row_kept(t: &Target, element: &Ref<TreeDataGridRow>, item: &Rc<RowModel>, index: usize) {
    assert!(Rc::ptr_eq(item, &t.items.get(index)));
    assert_eq!(row_at(&t.target.realized_elements(), index), *element);
    assert_data_contexts_are_items(&realized_rows(&t.target), &first_items(&t.items, 10));
}

#[test]
fn keeps_offscreen_focused_row_with_its_item_after_insert() {
    let _app = start_app();
    let t = create_target();
    let element = row_at(&t.target.realized_elements(), 5);
    let item = t.items.get(5);

    focus(&element);
    t.scroll.set_offset(Vector::new(0.0, 100.0));
    layout(&t.target);

    t.items.insert(0, RowModel::new(-1, "Inserted"));
    t.scroll.set_offset(Vector::default());
    layout(&t.target);

    assert_offscreen_focused_row_kept(&t, &element, &item, 6);
}

#[test]
fn keeps_offscreen_focused_row_with_its_item_after_remove() {
    let _app = start_app();
    let t = create_target();
    let element = row_at(&t.target.realized_elements(), 5);
    let item = t.items.get(5);

    focus(&element);
    t.scroll.set_offset(Vector::new(0.0, 100.0));
    layout(&t.target);

    t.items.remove_at(0);
    t.scroll.set_offset(Vector::default());
    layout(&t.target);

    assert_offscreen_focused_row_kept(&t, &element, &item, 4);
}

#[test]
fn keeps_offscreen_focused_row_with_its_item_after_move() {
    let _app = start_app();
    let t = create_target();
    let element = row_at(&t.target.realized_elements(), 5);
    let item = t.items.get(5);

    focus(&element);
    t.scroll.set_offset(Vector::new(0.0, 100.0));
    layout(&t.target);

    t.items.move_item(0, 8);
    t.scroll.set_offset(Vector::default());
    layout(&t.target);

    assert_offscreen_focused_row_kept(&t, &element, &item, 4);
}

fn titles(target: &TreeDataGridRowsPresenter) -> Vec<String> {
    realized_rows(target).iter().map(|row| model_of_row(row).title()).collect()
}

#[test]
fn handles_move_after_insert_without_duplicates() {
    // This tests the specific scenario: add an item at index 0, then move the item from index 2
    // to position 1. The bug was that after this sequence, indices 2 and 3 would become duplicates
    // of indices 0 and 1.
    let _app = start_app();
    let t = create_target_with(Options { item_count: 4, ..Options::default() });

    layout(&t.target);

    // Initial state: [Item0, Item1, Item2, Item3] at indices 0-3.
    assert_eq!(4, t.items.count());
    assert_eq!(vec!["Item 0", "Item 1", "Item 2", "Item 3"], titles(&t.target));

    // Step 1: add a new item at index 0.
    t.items.insert(0, RowModel::new(100, "Item 100"));
    layout(&t.target);

    // After insert: [Item100, Item0, Item1, Item2, Item3] at indices 0-4.
    assert_eq!(5, t.items.count());
    assert_eq!(vec!["Item 100", "Item 0", "Item 1", "Item 2", "Item 3"], titles(&t.target));

    // Step 2: move the item at index 2 (Item1) to position 1.
    t.items.move_item(2, 1);
    layout(&t.target);

    // After move: [Item100, Item1, Item0, Item2, Item3] at indices 0-4.
    assert_eq!(5, t.items.count());

    // Verify no duplicates - each element should have the correct data context.
    let realized_models: Vec<Rc<RowModel>> = realized_rows(&t.target).iter().map(|row| model_of_row(row)).collect();

    assert_eq!(vec!["Item 100", "Item 1", "Item 0", "Item 2", "Item 3"], titles(&t.target));

    // Verify all ids are unique (no duplicates).
    let mut unique_ids: Vec<i32> = realized_models.iter().map(|m| m.id.get()).collect();
    unique_ids.sort();
    unique_ids.dedup();
    assert_eq!(5, unique_ids.len());
}

#[test]
fn handles_remove_updates_subsequent_element_data() {
    // Test that when items are removed, elements after the removal get their data updated.
    let _app = start_app();
    let t = create_target_with(Options { item_count: 5, ..Options::default() });

    layout(&t.target);

    // Initial state: [Item0, Item1, Item2, Item3, Item4] at indices 0-4.
    assert_eq!(5, t.items.count());
    assert_eq!(vec!["Item 0", "Item 1", "Item 2", "Item 3", "Item 4"], titles(&t.target));

    // Remove Item1 at index 1.
    t.items.remove_at(1);
    layout(&t.target);

    // After remove: [Item0, Item2, Item3, Item4] at indices 0-3.
    assert_eq!(4, t.items.count());

    // Verify all elements have the correct data context (not stale data).
    let realized_models: Vec<Rc<RowModel>> = realized_rows(&t.target).iter().map(|row| model_of_row(row)).collect();

    assert_eq!(4, realized_models.len());
    assert_eq!(vec!["Item 0", "Item 2", "Item 3", "Item 4"], titles(&t.target));

    // Verify the ids match the titles (no stale data).
    assert_eq!(vec![0, 2, 3, 4], realized_models.iter().map(|m| m.id.get()).collect::<Vec<i32>>());
}

#[test]
fn handles_insert_updates_subsequent_element_data() {
    // Test that when items are inserted, elements after the insertion get their data updated.
    let _app = start_app();
    let t = create_target_with(Options { item_count: 4, ..Options::default() });

    layout(&t.target);

    // Initial state: [Item0, Item1, Item2, Item3] at indices 0-3.
    assert_eq!(4, t.items.count());
    assert_eq!(vec!["Item 0", "Item 1", "Item 2", "Item 3"], titles(&t.target));

    // Insert a new item at index 1.
    t.items.insert(1, RowModel::new(100, "Item 100"));
    layout(&t.target);

    // After insert: [Item0, Item100, Item1, Item2, Item3] at indices 0-4.
    assert_eq!(5, t.items.count());

    // Verify all elements have the correct data context (not stale data).
    let realized_models: Vec<Rc<RowModel>> = realized_rows(&t.target).iter().map(|row| model_of_row(row)).collect();

    assert_eq!(5, realized_models.len());
    assert_eq!(vec!["Item 0", "Item 100", "Item 1", "Item 2", "Item 3"], titles(&t.target));

    // Verify the ids match the titles (no stale data).
    assert_eq!(vec![0, 100, 1, 2, 3], realized_models.iter().map(|m| m.id.get()).collect::<Vec<i32>>());
}

fn tab_item_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TabItem>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &*TemplateBinding::new(HeaderedContentControl::header_property().as_property()),
        );
        presenter.bind_binding(
            ContentPresenter::content_template_property().as_property(),
            &*TemplateBinding::new(HeaderedContentControl::header_template_property().as_property()),
        );
        presenter.set_recognizes_access_key(true);
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn tab_control_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TabControl>(|_, scope| {
        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let items_presenter = items_presenter.register_in_name_scope(&**scope);

        let content_host = ContentPresenter::new();
        content_host.set_name(Some("PART_SelectedContentHost".to_string()));
        content_host.bind_binding(
            ContentPresenter::content_property().as_property(),
            &*TemplateBinding::new(TabControl::selected_content_property().as_property()),
        );
        content_host.bind_binding(
            ContentPresenter::content_template_property().as_property(),
            &*TemplateBinding::new(TabControl::selected_content_template_property().as_property()),
        );
        let content_host = content_host.register_in_name_scope(&**scope);

        let panel = StackPanel::new();
        panel.children().add(items_presenter);
        panel.children().add(content_host);
        panel.upcast()
    })
}

fn apply_tab_control_template(target: &TabControl) {
    target.apply_template();

    if let Some(presenter) = target.presenter() {
        presenter.apply_template();
    }

    for tab_item in logical_children_of::<TabItem>(target) {
        tab_item.set_template(Some(tab_item_template()));

        tab_item.apply_template();

        if let Some(presenter) = tab_item.presenter() {
            presenter.update_child();
        }
    }
}

#[test]
fn handles_adding_rows_while_detached_from_visual_tree() {
    let _app = start_app();
    let t = create_target_with(Options { item_count: 5, ..Options::default() });
    let test_window = t.root.clone();

    test_window.set_content(None);
    test_window.update_layout();

    let tab_item = TabItem::new();
    tab_item.set_content(Some(Control::boxed(&t.scroll)));
    let tab_control = TabControl::new();
    tab_control.items().add(Some(Control::boxed(&tab_item)));
    tab_control.items().add(Some(Control::boxed(TabItem::new())));
    tab_control.set_template(Some(tab_control_template()));

    apply_tab_control_template(&tab_control);

    test_window.set_content(Some(Control::boxed(&tab_control)));
    tab_control.apply_template();
    test_window.update_layout();

    run_jobs();

    tab_control.set_selected_index(1);

    layout(&t.target);

    for x in 5..10 {
        t.items.insert(1, RowModel::new(x, &format!("Item {x}")));
    }

    tab_control.set_selected_index(0);
    layout(&t.target);

    let indexes = get_realized_row_indexes(&t.target);
    let models: Vec<Rc<RowModel>> = realized_rows(&t.target).iter().map(|row| super::support::model_of::<RowModel>(row.model())).collect();

    let mut distinct_ids: Vec<i32> = models.iter().map(|m| m.id.get()).collect();
    distinct_ids.sort();
    distinct_ids.dedup();

    assert_eq!(10, indexes.len());
    assert_eq!(10, models.len());
    assert_eq!(10, distinct_ids.len());
}

#[test]
fn updates_star_column_actual_width() {
    let _app = start_app();
    let columns = column_list([
        TextColumn::<RowModel, i32>::new(
            Some(Rc::new("ID".to_string()) as BoxedValue),
            Accessor::member("Id", |x: &Rc<RowModel>| x.id.get()),
            Some(GridLength::STAR),
            None,
        ) as ColumnRef,
        TextColumn::<RowModel, Option<String>>::new(
            Some(Rc::new("Title".to_string()) as BoxedValue),
            Accessor::member("Title", |x: &Rc<RowModel>| x.title.borrow().clone()),
            Some(GridLength::STAR),
            None,
        ) as ColumnRef,
    ]);

    let _t = create_target_with(Options { columns: Some(columns.clone() as Rc<dyn IColumns>), ..Options::default() });

    for i in 0..columns.count() {
        assert_eq!(50.0, columns.get(i).actual_width());
    }
}

#[test]
fn brings_next_item_into_view() {
    let _app = start_app();
    let t = create_target();

    t.target.bring_into_view(10, None);
    layout(&t.target);

    assert_row_indexes(&t.target, 1, 10);
}

#[test]
fn bring_into_view_rejects_invalid_indexes_without_changing_viewport() {
    let _app = start_app();
    let t = create_target();
    let original_offset = t.scroll.offset();
    let original_rows = t.target.realized_elements();

    assert!(t.target.bring_into_view(-1, None).is_none());
    assert!(t.target.bring_into_view(t.items.count() as i32, None).is_none());
    layout(&t.target);

    assert_eq!(original_offset, t.scroll.offset());
    assert_eq!(original_rows, t.target.realized_elements());
}

#[test]
fn bring_into_view_does_nothing_when_presenter_is_effectively_invisible() {
    let _app = start_app();
    let t = create_target();
    t.scroll.set_is_visible(false);
    layout(&t.target);

    assert!(!t.target.is_effectively_visible());
    assert!(t.target.bring_into_view(50, None).is_none());
}

#[test]
fn is_visible_binding_persists_after_row_recycling() {
    let _app = start_app();
    let visibility_style = Style::with_setters(
        Selectors::of_type::<TreeDataGridRow>(),
        [Setter::new_binding_base(Visual::is_visible_property().as_property(), ReflectionBinding::new("IsRowVisible"))],
    );
    let t = create_target_with(Options { additional_styles: vec![visibility_style], ..Options::default() });
    let original = row(t.target.try_get_element(2));

    assert!(BindingOperations::get_binding_expression_base(&original, Visual::is_visible_property().as_property()).is_some());

    t.scroll.set_offset(Vector::new(0.0, 200.0));
    layout(&t.target);
    t.scroll.set_offset(Vector::default());
    layout(&t.target);

    let current = row(t.target.try_get_element(2));
    t.items.get(2).set_is_row_visible(false);
    layout(&t.target);

    assert!(BindingOperations::get_binding_expression_base(&current, Visual::is_visible_property().as_property()).is_some());
    assert!(!current.is_visible());
}

#[test]
fn handles_bringing_item_into_view_which_will_already_be_in_view_when_created() {
    let _app = start_app();
    let t = create_target();

    // Clear the items and do a layout to simulate starting from an empty state.
    let items = t.target.items();
    t.target.set_items(None);
    layout(&t.target);

    // Assign the items.
    t.target.set_items(items);

    // Now bring the first item into view before it's created. There was an issue here where the
    // presenter will wait for a viewport update which will never come because the item will be
    // placed in the existing viewport.
    t.target.bring_into_view(0, None);

    assert_row_indexes(&t.target, 0, 10);
}

#[test]
fn brings_partially_visible_new_item_into_view() {
    // Issue #77 of the reference.
    let _app = start_app();
    let t = create_target_with(Options { item_count: 9, root_size: Some(Size::new(100.0, 95.0)), ..Options::default() });

    assert_row_indexes(&t.target, 0, 9);

    t.items.add(RowModel::new(100, "New Item"));
    t.target.bring_into_view(9, None);
    layout(&t.target);

    assert_row_indexes(&t.target, 0, 10);
}

#[test]
fn assigns_row_data_contexts() {
    let _app = start_app();
    let t = create_target();
    let last_row = realized_rows(&t.target).last().cloned().expect("a row");

    for i in 0..10 {
        assert!(same_model(&t.items.get(i), row_at(&t.target.realized_elements(), i).data_context().as_ref()));
    }

    t.items.remove_range(0, 99);
    layout(&t.target);

    assert_eq!(-1, last_row.row_index());
    assert!(last_row.data_context().is_none());
}
