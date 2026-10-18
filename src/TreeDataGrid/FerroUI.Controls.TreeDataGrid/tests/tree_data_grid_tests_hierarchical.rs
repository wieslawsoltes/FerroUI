//! `TreeDataGridTests_Hierarchical.cs`: the grid over a hierarchical source: the initial rows and
//! cells, expanding, adding, moving and removing models, the subscriptions of the realized rows to
//! their models, the recycling of templates and cells, reassigning and resetting the items while
//! scrolled, and the selection of a removed model.
//!
//! The reference's classic `HierarchicalTreeDataGridSource<Model>` with its classic columns is the
//! core's hierarchical source holding the view columns of this crate (`crate::models`), set on the
//! grid as its `Source`.

use crate::models::{
    CellRef, HierarchicalExpanderColumn, ICell, IColumns, IViewColumn, TemplateCell, TemplateColumn, TextColumn,
};
use crate::presentation::TreeDataGridPresentation;
use crate::primitives::{
    TemplateCellDataContext, TreeDataGridCell, TreeDataGridExpanderCell, TreeDataGridRow, TreeDataGridRowsPresenter,
    TreeDataGridTemplateCell,
};
use crate::tests::support::*;
use crate::TreeDataGrid;
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{Maybe, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model as FerroModel};
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{ferro_model, BoxedValue, Ref, Vector};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{FuncDataTemplate, IDataTemplate};
use ferroui_controls::{Border, Control, ScrollViewer, Window};
use ferroui_treedatagrid_core::index_path;
use ferroui_treedatagrid_core::models::{ColumnRef as CoreColumnRef, IExpander, IRow, IRows, ListSortDirection};
use ferroui_treedatagrid_core::selection::ITreeSelectionModel;
use ferroui_treedatagrid_core::{Accessor, HierarchicalTreeDataGridSource, ITreeDataGridSource, IndexPath, Models};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------------------------

/// `Model : NotifyingBase` of the tests: an id, a title and optional children, whose
/// `PropertyChanged` counts its subscribers.
struct Model {
    id: i32,
    title: Option<String>,
    children: RefCell<Option<Rc<FerroList<Rc<Model>>>>>,
    notifier: TestNotifier,
}

ferro_model!(Model, |b| b
    .notify_property_changed()
    .read_only::<Value<i32>>("Id", |m| m.id)
    .read_only::<Maybe<String>>("Title", |m| m.title.clone()));

impl INotifyPropertyChanged for Model {
    fn property_changed(&self) -> &Event<str> {
        self.notifier.event()
    }
}

impl Model {
    /// `new Model { Id = id, Title = title }`.
    fn new(id: i32, title: &str) -> Rc<Self> {
        Self::create(id, title, None)
    }

    /// `new Model { Id = id, Title = title, Children = new List<Model>(children) }`.
    fn with_children(id: i32, title: &str, children: Vec<Rc<Model>>) -> Rc<Self> {
        Self::create(id, title, Some(Rc::new(FerroList::from_items(children))))
    }

    fn create(id: i32, title: &str, children: Option<Rc<FerroList<Rc<Model>>>>) -> Rc<Self> {
        <Model as FerroModel>::new_model(Self {
            id,
            title: Some(title.to_string()),
            children: RefCell::new(children),
            notifier: TestNotifier::new(),
        })
    }

    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    fn children(&self) -> Option<Rc<FerroList<Rc<Model>>>> {
        self.children.borrow().clone()
    }

    /// `Children = ...` (the reference's auto-property raises no change notification).
    fn set_children(&self, value: Option<Rc<FerroList<Rc<Model>>>>) {
        *self.children.borrow_mut() = value;
    }

    /// `PropertyChangedSubscriberCount()`.
    fn property_changed_subscriber_count(&self) -> usize {
        self.notifier.property_changed_subscriber_count()
    }
}

/// The subscriptions the cells of a realized row make to its model.
///
/// The reference counts 2: the text cells of the inner `ID` column and of the `Title` column. Its
/// has-children selector `x => true` is an expression that reads no member, so it observes
/// nothing. In the port, the has-children selector is an accessor without a member name or path,
/// which the show-expander binding of the expander cell follows through the model's
/// `PropertyChanged` (an accessor cannot tell a constant from a computed value), so the expander
/// cell adds one subscription.
const REALIZED_ROW_SUBSCRIPTIONS: usize = 3;

/// The subscription of an expanded row to its model.
///
/// The reference's classic rows do not subscribe to their models (0). The port's rows are the
/// core's, whose expanded rows follow their model's `PropertyChanged` to refresh their children,
/// whether the row is realized or not.
const EXPANDED_ROW_SUBSCRIPTIONS: usize = 1;

// ---------------------------------------------------------------------------------------------
// Target
// ---------------------------------------------------------------------------------------------

struct Target {
    target: Ref<TreeDataGrid>,
    source: Rc<HierarchicalTreeDataGridSource<Model>>,
    /// The items of the source (`source.Items` as the list of the test).
    items: Rc<FerroList<Rc<Model>>>,
    window: Ref<Window>,
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// `x => x.Children`.
fn child_models(model: &Rc<Model>) -> Option<Models<Model>> {
    model.children().map(|children| children as Models<Model>)
}

/// `x => true`.
fn has_children_always() -> Accessor<Model, bool> {
    Accessor::new(|_: &Rc<Model>| true)
}

/// `new TextColumn<Model, string?>("Title", x => x.Title)`.
fn title_column() -> CoreColumnRef<Model> {
    TextColumn::<Model, Option<String>>::new(
        header("Title"),
        Accessor::member("Title", |x: &Rc<Model>| x.title.clone()),
        None,
        None,
    ) as CoreColumnRef<Model>
}

/// The default columns of `CreateTarget`: an expander column around the `ID` text column, and
/// the `Title` text column.
fn default_columns() -> Vec<CoreColumnRef<Model>> {
    let id = TextColumn::<Model, i32>::new(header("ID"), Accessor::member("Id", |x: &Rc<Model>| x.id), None, None);
    vec![
        HierarchicalExpanderColumn::new(
            id as Rc<dyn IViewColumn<Model>>,
            child_models,
            Some(has_children_always()),
            None,
        ) as CoreColumnRef<Model>,
        title_column(),
    ]
}

/// `CreateModels(titlePrefix, count, firstIndex, firstId)`.
fn create_models(title_prefix: &str, count: i32, first_index: i32, first_id: i32) -> Vec<Rc<Model>> {
    (0..count)
        .map(|x| Model::new(first_id + first_index + x, &format!("{title_prefix}{}", first_index + x)))
        .collect()
}

fn create_target() -> Target {
    create_target_with(None, true)
}

fn create_target_with(columns: Option<Vec<CoreColumnRef<Model>>>, run_layout: bool) -> Target {
    let items = Rc::new(FerroList::from_items([
        Model::with_children(0, "Root 0", create_models("Item 0-", 100, 0, 100)),
        Model::new(1, "Root 1"),
    ]));

    let columns = columns.unwrap_or_else(default_columns);

    let source = HierarchicalTreeDataGridSource::<Model>::new(items.clone());
    for column in columns {
        source.columns().add(column);
    }

    let target = TreeDataGrid::new();
    target.set_template(Some(tree_data_grid_template(0.0)));
    target.set_source(source_ref::<Model, _>(&source));

    let window = test_window(Some(target.clone().upcast()), None);
    window.styles().add(tree_data_grid_expander_cell_style());
    window.styles().add(tree_data_grid_template_cell_style());
    window.styles().add(Style::with_setters(
        Selectors::is::<TreeDataGridRow>(),
        [Setter::new(TemplatedControl::template_property(), Some(tree_data_grid_row_template()))],
    ));
    window.styles().add(Style::with_setters(
        Selectors::is::<TreeDataGridCell>(),
        [Setter::new(Layoutable::height_property(), 10.0)],
    ));

    if run_layout {
        window.update_layout();
    }
    run_jobs();

    Target { target, source, items, window }
}

/// `InitialLayout(target)`.
fn initial_layout(target: &TreeDataGrid) {
    target.update_layout();
}

fn rows_presenter(target: &TreeDataGrid) -> Ref<TreeDataGridRowsPresenter> {
    target.rows_presenter().expect("a rows presenter")
}

fn scroll(target: &TreeDataGrid) -> Ref<ScrollViewer> {
    target.scroll().expect("a scroll viewer")
}

/// `(TreeDataGridRow)element!`.
fn as_row(element: Option<Ref<Control>>) -> Ref<TreeDataGridRow> {
    element.and_then(|element| element.cast::<TreeDataGridRow>()).expect("a row")
}

/// `Assert.IsType<TreeDataGridExpanderCell>(element)`.
fn as_expander_cell(element: Option<Ref<Control>>) -> Ref<TreeDataGridExpanderCell> {
    element.and_then(|element| element.cast::<TreeDataGridExpanderCell>()).expect("an expander cell")
}

/// `source.Rows[rowIndex].Model`.
fn row_model(source: &HierarchicalTreeDataGridSource<Model>, row_index: i32) -> Option<BoxedValue> {
    source.rows().get(row_index as usize).model()
}

/// The row indexes of the realized rows (`RealizedElements.OfType<TreeDataGridRow>().Select(x => x.RowIndex)`).
fn realized_row_indexes(presenter: &TreeDataGridRowsPresenter) -> Vec<i32> {
    presenter
        .realized_elements()
        .into_iter()
        .flatten()
        .filter_map(|element| element.cast::<TreeDataGridRow>())
        .map(|row| row.row_index())
        .collect()
}

/// Whether two cell models are the same object (`Assert.Same`).
fn same_cell(a: &CellRef, b: Option<&CellRef>) -> bool {
    b.is_some_and(|b| std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)))
}

/// The models of the first and the last realized rows, for the reassign and reset tests.
fn first_and_last_realized_models(t: &Target) -> (Rc<Model>, Rc<Model>) {
    let elements = rows_presenter(&t.target).realized_elements();
    let first_row = as_row(elements.first().cloned().flatten());
    let last_row = as_row(elements.last().cloned().flatten());
    let first_row_model = model_of::<Model>(row_model(&t.source, first_row.row_index()));
    let last_row_model = model_of::<Model>(row_model(&t.source, last_row.row_index()));
    (first_row_model, last_row_model)
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[test]
fn should_display_initial_row_and_cells() {
    let _app = start_app();
    let t = create_target();

    assert!(t.target.rows_presenter().is_some());

    let presenter = rows_presenter(&t.target);
    let rows = visual_children_of::<TreeDataGridRow>(&presenter);

    // `Cast<TreeDataGridRow>()`: every visual child is a row.
    assert_eq!(presenter.get_visual_children().len(), rows.len());
    assert_eq!(2, rows.len());

    for row in &rows {
        let cells_presenter = row.cells_presenter().expect("a cells presenter");
        let cells = visual_children_of::<TreeDataGridCell>(&cells_presenter);
        assert_eq!(cells_presenter.get_visual_children().len(), cells.len());
        assert_eq!(2, cells.len());
    }

    t.window.close();
}

#[test]
fn should_display_expanded_root_node() {
    let _app = start_app();
    let t = create_target();

    assert!(t.target.rows_presenter().is_some());
    let presenter = rows_presenter(&t.target);
    assert_eq!(2, presenter.realized_elements().len());
    assert_eq!(2, presenter.get_visual_children().len());

    t.source.expand(&IndexPath::new(0));

    assert_eq!(102, t.source.rows().count());
    assert_eq!(1, presenter.realized_elements().len());
    assert_eq!(2, presenter.get_visual_children().len());

    layout(&t.target);

    assert_eq!(10, presenter.realized_elements().len());

    t.window.close();
}

#[test]
fn should_display_added_root_node() {
    let _app = start_app();
    let t = create_target();

    layout(&t.target);
    t.items.add(Model::new(-1, "Added"));
    layout(&t.target);

    let presenter = rows_presenter(&t.target);
    assert_eq!(3, presenter.realized_elements().len());
    assert_eq!(3, presenter.get_visual_children().len());

    t.window.close();
}

#[test]
fn should_display_added_child_node() {
    let _app = start_app();
    let t = create_target();
    let children = Rc::new(FerroList::from_items([Model::new(-1, "First")]));
    t.items.get(1).set_children(Some(children.clone()));

    layout(&t.target);
    t.source.expand(&IndexPath::new(1));
    layout(&t.target);
    children.add(Model::new(-2, "Second"));
    layout(&t.target);

    let presenter = rows_presenter(&t.target);
    assert_eq!(4, presenter.realized_elements().len());
    assert_eq!(4, presenter.get_visual_children().len());

    t.window.close();
}

#[test]
fn moving_expanded_children_preserves_realized_row_identity() {
    let _app = start_app();
    let t = create_target();
    let children = t.items.get(0).children().expect("children");

    t.source.expand(&IndexPath::new(0));
    layout(&t.target);

    let presenter = rows_presenter(&t.target);

    // `ToDictionary(x => (Model)x.DataContext!)` and the subscriber counts of its keys.
    let rows_by_item: Vec<(Rc<Model>, Ref<TreeDataGridRow>, usize)> = realized_rows(&presenter.realized_elements())
        .into_iter()
        .map(|row| {
            let item = model_of::<Model>(row.data_context());
            let count = item.property_changed_subscriber_count();
            (item, row, count)
        })
        .collect();

    children.move_item(1, 6);
    layout(&t.target);

    let realized = realized_rows(&presenter.realized_elements());
    let mut row_indexes: Vec<i32> = realized.iter().map(|x| x.row_index()).collect();
    row_indexes.sort();
    row_indexes.dedup();
    assert_eq!(realized.len(), row_indexes.len());

    for row in &realized {
        let item = model_of::<Model>(row.data_context());
        let (_, row_before, count_before) = rows_by_item
            .iter()
            .find(|(model, _, _)| Rc::ptr_eq(model, &item))
            .expect("a row realized before the move");
        assert_eq!(*row_before, *row);
        assert_eq!(*count_before, item.property_changed_subscriber_count());
        assert!(has_visual_child(&presenter, row));
        assert!(has_logical_child(&presenter, row));
    }

    t.window.close();
}

#[test]
fn collection_edit_preserves_expanded_row_and_expander_cell_with_their_model() {
    let _app = start_app();
    let t = create_target();
    let retained_item = t.items.get(0);

    t.source.expand(&IndexPath::new(0));
    layout(&t.target);

    let retained_row = t.target.try_get_row(0).expect("a row");
    let retained_expander = as_expander_cell(retained_row.try_get_cell(0));
    let retained_cell_model = retained_expander.model().expect("a cell model");
    // `Assert.IsAssignableFrom<IExpanderCell>(retainedExpander.Model)`.
    assert!(retained_cell_model.as_expander_cell().is_some());

    assert!(retained_expander.show_expander());
    assert!(retained_expander.is_expanded());

    let assert_retained_expander = |row_index: i32, is_expanded: bool| {
        let row = t.target.try_get_row(row_index).expect("a row");
        let expander = as_expander_cell(row.try_get_cell(0));
        assert!(same_model(&retained_item, row_model(&t.source, row_index).as_ref()));
        assert_eq!(retained_row, row);
        assert!(same_model(&retained_item, row.data_context().as_ref()));
        assert_eq!(retained_expander, expander);
        assert!(same_cell(&retained_cell_model, expander.model().as_ref()));
        assert_eq!(row_index, expander.row_index());
        assert!(expander.show_expander());
        assert_eq!(is_expanded, expander.is_expanded());
    };

    t.items.insert(0, Model::new(-1, "Inserted"));
    layout(&t.target);

    assert_retained_expander(1, true);

    retained_expander.set_is_expanded(false);
    layout(&t.target);
    assert_retained_expander(1, false);
    assert!(!retained_cell_model.as_expander_cell().expect("an expander cell model").is_expanded());

    retained_expander.set_is_expanded(true);
    layout(&t.target);
    assert_retained_expander(1, true);
    assert!(retained_cell_model.as_expander_cell().expect("an expander cell model").is_expanded());

    t.items.remove_at(0);
    layout(&t.target);

    assert_retained_expander(0, true);

    t.window.close();
}

#[test]
fn row_indexes_should_be_correct_after_expanding_node_while_scrolled() {
    let _app = start_app();
    let t = create_target();
    let children = Rc::new(FerroList::from_items([Model::new(-1, "First")]));
    t.items.get(0).children().expect("children").get(1).set_children(Some(children));

    t.source.expand(&IndexPath::new(0));
    scroll(&t.target).set_offset(Vector::new(0.0, 20.0));
    layout(&t.target);

    let presenter = rows_presenter(&t.target);
    let row_indexes = realized_row_indexes(&presenter);

    assert_eq!((2..12).collect::<Vec<i32>>(), row_indexes);

    t.source.expand(&index_path![0, 1]);
    layout(&t.target);

    let row_indexes = realized_row_indexes(&presenter);

    assert_eq!((2..12).collect::<Vec<i32>>(), row_indexes);

    t.window.close();
}

#[test]
fn should_subscribe_to_models_for_initial_rows() {
    let _app = start_app();
    let t = create_target();

    for i in 0..t.items.count() {
        assert_eq!(REALIZED_ROW_SUBSCRIPTIONS, t.items.get(i).property_changed_subscriber_count(), "item {i}");
    }

    t.window.close();
}

#[test]
fn should_subscribe_to_models_for_expanded_rows() {
    let _app = start_app();
    let t = create_target();

    t.source.expand(&IndexPath::new(0));
    layout(&t.target);

    // The reference: 2 for the realized, expanded first root and 0 for the second root, which is
    // scrolled out of view.
    assert_eq!(
        REALIZED_ROW_SUBSCRIPTIONS + EXPANDED_ROW_SUBSCRIPTIONS,
        t.items.get(0).property_changed_subscriber_count()
    );
    assert_eq!(0, t.items.get(1).property_changed_subscriber_count());

    let children = t.items.get(0).children().expect("children");
    for i in 0..children.count() {
        let expected = if i < 9 { REALIZED_ROW_SUBSCRIPTIONS } else { 0 };
        assert_eq!(expected, children.get(i).property_changed_subscriber_count(), "child {i}");
    }

    t.window.close();
}

#[test]
fn should_subscribe_to_correct_models_after_scrolling_down_one_row() {
    let _app = start_app();
    let t = create_target();

    t.source.expand(&IndexPath::new(0));
    layout(&t.target);
    scroll(&t.target).set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    // The reference: 0 for the first root, whose row is no longer realized; its row is still
    // expanded, which the port's core row follows.
    assert_eq!(EXPANDED_ROW_SUBSCRIPTIONS, t.items.get(0).property_changed_subscriber_count());
    assert_eq!(0, t.items.get(1).property_changed_subscriber_count());

    let children = t.items.get(0).children().expect("children");
    for i in 0..children.count() {
        let expected = if i < 10 { REALIZED_ROW_SUBSCRIPTIONS } else { 0 };
        assert_eq!(expected, children.get(i).property_changed_subscriber_count(), "child {i}");
    }

    t.window.close();
}

#[test]
fn scrolling_should_not_rebuild_templates_in_expander_columns() {
    let _app = start_app();
    let instantiations = Rc::new(Cell::new(0));

    // `Control Template(Model model, INameScope ns)`.
    let template = {
        let instantiations = instantiations.clone();
        FuncDataTemplate::for_type::<Model>(
            move |_, _| {
                instantiations.set(instantiations.get() + 1);
                Some(Border::new().upcast())
            },
            true,
        ) as Rc<dyn IDataTemplate>
    };

    let columns = vec![
        HierarchicalExpanderColumn::new(
            TemplateColumn::<Model>::new(header("ID"), template, None, None, None) as Rc<dyn IViewColumn<Model>>,
            child_models,
            Some(has_children_always()),
            None,
        ) as CoreColumnRef<Model>,
        title_column(),
    ];

    // Create the TreeDataGrid but don't do an initial layout.
    let t = create_target_with(Some(columns), false);

    // Expand the first root and do the initial layout now.
    instantiations.set(0);
    t.source.expand(&IndexPath::new(0));
    initial_layout(&t.target);
    assert!(instantiations.get() > 0);
    let initial_instantiations = instantiations.get();

    // Scroll down a row.
    scroll(&t.target).set_offset(Vector::new(0.0, 10.0));
    layout(&t.target);

    // Template should have been recycled and not rebuilt.
    assert_eq!(initial_instantiations, instantiations.get());
    let presenter = rows_presenter(&t.target);
    assert_eq!(10, presenter.realized_elements().len());

    for i in 0..10 {
        let row = as_row(presenter.realized_elements()[i].clone());
        let cells_presenter = row.cells_presenter().expect("a cells presenter");
        let cell = as_expander_cell(cells_presenter.realized_elements()[0].clone());
        let inner = cell.find_descendant_of_type::<TreeDataGridTemplateCell>(false).expect("a template cell");
        // `(TemplateCell)inner.DataContext!`: the port's template cell holds its cell model in a
        // `TemplateCellDataContext`.
        let inner_data_context = inner.data_context().expect("a data context");
        let inner_model = inner_data_context
            .downcast_ref::<TemplateCellDataContext>()
            .expect("the data context of a template cell")
            .0
            .clone();
        let inner_model = inner_model.as_any().downcast_ref::<TemplateCell>().expect("a template cell model");
        let expected_model = row_model(&t.source, i as i32 + 1);

        assert!(same_value(expected_model.as_ref(), row.data_context().as_ref()));
        assert!(same_value(expected_model.as_ref(), cell.data_context().as_ref()));
        assert!(same_value(expected_model.as_ref(), inner_model.value().as_ref()));
    }

    t.window.close();
}

#[test]
fn should_unsubscribe_from_models_when_detached_from_logical_tree() {
    let _app = start_app();
    let t = create_target();

    // `((Window)target.Parent!).Content = null`.
    t.target
        .parent()
        .and_then(|parent| parent.cast::<Window>())
        .expect("the window of the grid")
        .set_content(None);

    for i in 0..t.items.count() {
        assert_eq!(0, t.items.get(i).property_changed_subscriber_count(), "item {i}");
    }

    t.window.close();
}

#[test]
fn should_hide_expander_when_node_with_no_children_expanded() {
    let _app = start_app();
    let t = create_target();
    let cell = t.target.try_get_cell(0, 1);
    let expander = as_expander_cell(cell);

    assert!(!expander.is_expanded());
    assert!(expander.show_expander());

    expander.set_is_expanded(true);

    assert!(!expander.is_expanded());
    assert!(!expander.show_expander());

    t.window.close();
}

#[test]
fn can_reassign_items_when_displaying_child_items_followed_by_root_items() {
    let _app = start_app();
    let t = create_target();
    let cell = t.target.try_get_cell(0, 0);
    let _expander = as_expander_cell(cell);

    // Add a a few more root items.
    t.items.add_range(create_models("Root ", 5, 2, 100));

    // Expand the first root item and scroll down such that we're displaying some children
    // of the first root item together with subsequent root items.
    t.source.expand(&IndexPath::new(0));
    layout(&t.target);
    scroll(&t.target).set_offset(Vector::new(0.0, 1970.0));
    layout(&t.target);

    let (first_row_model, last_row_model) = first_and_last_realized_models(&t);

    assert_eq!(Some("Item 0-96"), first_row_model.title());
    assert_eq!(Some("Root 6"), last_row_model.title());

    // Replace the items with a single item.
    t.source.set_items(observable([Model::new(0, "Root 0")]));

    layout(&t.target);

    let first_row = as_row(rows_presenter(&t.target).realized_elements()[0].clone());
    assert_eq!(0, first_row.row_index());
    assert_eq!(Vector::new(0.0, 0.0), scroll(&t.target).offset());

    t.window.close();
}

#[test]
fn can_reassign_items_when_displaying_grandchild_items_followed_by_root_items() {
    let _app = start_app();
    let t = create_target();
    let cell = t.target.try_get_cell(0, 0);
    let _expander = as_expander_cell(cell);

    // Add a a few more root items.
    t.items.add_range(create_models("Root ", 5, 2, 100));

    // Add some grandchildren.
    t.items.get(0).children().expect("children").add_range(create_models("Item 0-0-", 100, 0, 100));

    // Expand the first child item and scroll down such that we're displaying some children
    // of the first root item together with subsequent root items.
    t.source.expand(&index_path![0, 0]);
    layout(&t.target);
    scroll(&t.target).set_offset(Vector::new(0.0, 1970.0));
    layout(&t.target);

    let (first_row_model, last_row_model) = first_and_last_realized_models(&t);

    assert_eq!(Some("Item 0-0-96"), first_row_model.title());
    assert_eq!(Some("Root 6"), last_row_model.title());

    // Replace the items with a single item.
    t.source.set_items(observable([Model::new(0, "Root 0")]));

    layout(&t.target);

    let first_row = as_row(rows_presenter(&t.target).realized_elements()[0].clone());
    assert_eq!(0, first_row.row_index());
    assert_eq!(Vector::new(0.0, 0.0), scroll(&t.target).offset());

    t.window.close();
}

#[test]
fn can_reset_items_when_displaying_child_items_followed_by_root_items() {
    let _app = start_app();
    let t = create_target();
    let cell = t.target.try_get_cell(0, 0);
    let _expander = as_expander_cell(cell);

    // Add a a few more root items.
    t.items.add_range(create_models("Root ", 5, 2, 100));

    // Expand the first root item and scroll down such that we're displaying some children
    // of the first root item together with subsequent root items.
    t.source.expand(&IndexPath::new(0));
    layout(&t.target);
    scroll(&t.target).set_offset(Vector::new(0.0, 1970.0));
    layout(&t.target);

    let (first_row_model, last_row_model) = first_and_last_realized_models(&t);

    assert_eq!(Some("Item 0-96"), first_row_model.title());
    assert_eq!(Some("Root 6"), last_row_model.title());

    // Clear the items.
    t.items.clear();

    layout(&t.target);

    assert!(rows_presenter(&t.target).realized_elements().is_empty());
    assert_eq!(Vector::new(0.0, 0.0), scroll(&t.target).offset());

    t.window.close();
}

#[test]
fn can_reset_child_items_when_displaying_grandchild_items_followed_by_root_items() {
    let _app = start_app();
    let t = create_target();
    let cell = t.target.try_get_cell(0, 0);
    let _expander = as_expander_cell(cell);

    // Add a a few more root items.
    t.items.add_range(create_models("Root ", 5, 2, 100));

    // Add some grandchildren.
    t.items.get(0).children().expect("children").add_range(create_models("Item 0-0-", 100, 0, 100));

    // Expand the first child item and scroll down such that we're displaying some children
    // of the first root item together with subsequent root items.
    t.source.expand(&index_path![0, 0]);
    layout(&t.target);
    scroll(&t.target).set_offset(Vector::new(0.0, 1970.0));
    layout(&t.target);

    let (first_row_model, last_row_model) = first_and_last_realized_models(&t);

    assert_eq!(Some("Item 0-0-96"), first_row_model.title());
    assert_eq!(Some("Root 6"), last_row_model.title());

    // Clear the child items.
    t.items.get(0).children().expect("children").clear();

    layout(&t.target);

    let first_row = as_row(rows_presenter(&t.target).realized_elements()[0].clone());
    assert_eq!(0, first_row.row_index());
    assert_eq!(Vector::new(0.0, 0.0), scroll(&t.target).offset());

    t.window.close();
}

#[test]
fn can_remove_selected_item() {
    let _app = start_app();
    let t = create_target();

    t.source.expand(&index_path![0, 0]);
    layout(&t.target);
    let row_selection = t.target.row_selection().expect("a row selection");
    row_selection.select(&index_path![0, 3]);

    assert_eq!(index_path![0, 3], row_selection.selected_index());

    t.items.get(0).children().expect("children").remove_at(3);

    assert_eq!(IndexPath::new(-1), row_selection.selected_index());

    t.window.close();
}

#[test]
fn can_remove_selected_item_sorted() {
    let _app = start_app();
    let t = create_target();

    // `target.Source!.SortBy(target.Columns![0], ListSortDirection.Descending)`: the grid's source
    // is presented by its presentation, which sorts the source by a column of the grid.
    let column = t.target.columns().expect("the columns of the grid").get(0);
    t.target.presentation().expect("a presentation").sort_by(&column, ListSortDirection::Descending);

    t.source.expand(&index_path![0, 0]);
    layout(&t.target);
    let row_selection = t.target.row_selection().expect("a row selection");
    row_selection.select(&index_path![0, 3]);

    assert_eq!(index_path![0, 3], row_selection.selected_index());

    t.items.get(0).children().expect("children").remove_at(3);

    assert_eq!(IndexPath::new(-1), row_selection.selected_index());

    t.window.close();
}

#[test]
fn should_recycle_focused_cell_when_row_collapsed() {
    // Issue #210.
    let _app = start_app();
    let t = create_target();

    t.source.expand(&IndexPath::new(0));
    layout(&t.target);

    assert_eq!(10, rows_presenter(&t.target).realized_elements().len());
    let row = t.target.try_get_row(1).expect("a row");
    let cell = as_expander_cell(row.try_get_cell(0));

    assert_eq!(1, cell.row_index());

    let _ = cell.focus();

    t.source.collapse(&IndexPath::new(0));

    assert_eq!(-1, row.row_index());

    // At this point, the cell should have been recycled and should have a row index of -1,
    // otherwise it can may recyled in a subsequent layout pass and the cell will not
    // be updated correctly.
    assert_eq!(-1, cell.row_index());

    t.window.close();
}
