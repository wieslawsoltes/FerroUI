//! Tests of the table view.
//!
//! Plain arrays of the reference tests are non-notifying items sources and
//! the person of the reference tests is a model type. The control used as a
//! view model in one test is the data context as a boxed control.

use crate::presenters::{
    ItemsPresenter, ScrollContentPresenter, TableViewCellsPresenter, TableViewColumnHeadersPresenter,
};
use crate::primitives::TemplatedControl;
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::test_support::{string_of, test_scope, TestRoot};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    reference_equals, AssignedBinding, Border, Control, Dock, DockPanel, GridLength, GridUnitType, ItemsControl,
    ItemsSource, Panel, ScrollViewer, TableView, TableViewCell, TableViewColumn, TableViewColumnHeader, TableViewRow,
    TextBlock,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{Maybe, Value};
use ferroui_base::data::model::Model;
use ferroui_base::data::{BindingBase, BindingMode, IndexerBinding, ReflectionBinding};
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style};
use ferroui_base::{ferro_model, BoxedValue, Ref, Size, StyledElement};
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

struct Person {
    name: String,
    nickname: Option<String>,
}

ferro_model!(Person, |b| b
    .read_only::<Value<String>>("Name", |person| person.name.clone())
    .read_only::<Maybe<String>>("Nickname", |person| person.nickname.clone()));

impl Person {
    fn new(name: &str, nickname: Option<&str>) -> Rc<Self> {
        Model::new_model(Self { name: name.to_string(), nickname: nickname.map(str::to_string) })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A running unit test application with the text services of the tests
/// registered over its services.
struct AppScope {
    // Dropped in this order: the text services first.
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

fn start() -> AppScope {
    let app = UnitTestApplication::start(TestServices::mock_platform_render_interface());
    AppScope { _text: TextTestScope::new(), _app: app }
}

fn strs(values: &[&str]) -> ItemsSource {
    ItemsSource::from_strs(values.iter().copied())
}

fn people(values: &[&Rc<Person>]) -> ItemsSource {
    ItemsSource::from_items(values.iter().map(|person| Some(Rc::clone(person) as BoxedValue)))
}

fn create_target(items: ItemsSource) -> Ref<TableView> {
    let target = TableView::new();
    target.set_template(Some(table_view_template()));
    target.set_item_container_theme(Some(table_view_row_theme()));
    target.set_items_source(Some(items));
    target
}

fn column() -> Ref<TableViewColumn> {
    TableViewColumn::new()
}

fn star_column() -> Ref<TableViewColumn> {
    let column = TableViewColumn::new();
    column.set_width(GridLength::new(1.0, GridUnitType::Star));
    column
}

fn binding(path: &str) -> Option<AssignedBinding> {
    let binding: Rc<dyn BindingBase> = ReflectionBinding::new(path);
    Some(AssignedBinding::new(binding))
}

fn text_block_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(|_| true, |_, _| Some(TextBlock::new().upcast()), false)
}

fn rows(target: &TableView) -> Vec<Ref<TableViewRow>> {
    target
        .get_realized_containers()
        .into_iter()
        .map(|container| container.cast::<TableViewRow>().expect("the container is not a table view row"))
        .collect()
}

fn single_row(target: &TableView) -> Ref<TableViewRow> {
    let mut rows = rows(target);
    assert_eq!(rows.len(), 1);
    rows.remove(0)
}

fn get_cells_presenter(row: &TableViewRow) -> Ref<TableViewCellsPresenter> {
    row.get_visual_descendants()
        .find_map(|visual| visual.cast::<TableViewCellsPresenter>())
        .expect("the row has no cells presenter")
}

fn get_column_headers_presenter(target: &TableView) -> Ref<TableViewColumnHeadersPresenter> {
    target
        .get_visual_descendants()
        .find_map(|visual| visual.cast::<TableViewColumnHeadersPresenter>())
        .expect("the table view has no column headers presenter")
}

fn cell(presenter: &TableViewCellsPresenter, index: usize) -> Ref<TableViewCell> {
    presenter.children().get(index).cast::<TableViewCell>().expect("the child is not a cell")
}

fn header(presenter: &TableViewColumnHeadersPresenter, index: usize) -> Ref<TableViewColumnHeader> {
    presenter.children().get(index).cast::<TableViewColumnHeader>().expect("the child is not a column header")
}

/// The children of a panel as logical elements.
fn children_of(panel: &Panel) -> Vec<Ref<StyledElement>> {
    panel.children().snapshot().iter().map(|child| child.clone().upcast()).collect()
}

fn logical_children(row: &TableViewRow) -> Vec<Ref<StyledElement>> {
    StyledElement::logical_children(row).to_vec()
}

fn content_string(cell: &TableViewCell) -> Option<String> {
    cell.content().as_ref().and_then(string_of)
}

fn prepare(target: &Ref<TableView>) -> Ref<TestRoot> {
    prepare_with(target, 300.0, 200.0)
}

fn prepare_with(target: &Ref<TableView>, width: f64, height: f64) -> Ref<TestRoot> {
    target.set_width(width);
    target.set_height(height);
    let root = TestRoot::with_child(target);
    root.execute_initial_layout_pass();
    root
}

fn layout(control: &Control) {
    if let Some(layout_manager) = control.get_layout_manager() {
        layout_manager.execute_layout_pass();
    }
}

fn table_view_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TableView>(|parent, scope| {
        let headers = TableViewColumnHeadersPresenter::new();
        DockPanel::set_dock(&headers, Dock::Top);

        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &IndexerBinding::new(parent.clone().upcast(), property, BindingMode::OneWay));

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        scroll_viewer.set_template(Some(scroll_viewer_template()));
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));

        let panel = DockPanel::new();
        panel.children().add(headers);
        panel.children().add(scroll_viewer.register_in_name_scope(&**scope));
        panel.upcast()
    })
}

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));

        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.upcast()
    })
}

fn table_view_row_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        TableViewRow::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(row_template()))],
    )
}

fn row_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TableViewRow>(|_, scope| {
        let presenter = TableViewCellsPresenter::new();
        presenter.set_name(Some("PART_CellsPresenter".to_string()));
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn container_for_each_item_is_table_view_row() {
    let _app = start();

    let target = create_target(strs(&["Foo", "Bar", "Baz"]));

    let _root = prepare(&target);

    let containers = target.get_realized_containers();
    assert_eq!(containers.len(), 3);
    for container in &containers {
        assert_eq!(container.get_type(), TableViewRow::TYPE);
    }
}

#[test]
fn row_has_one_cell_per_column() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    target.columns().add(column());
    target.columns().add(column());
    target.columns().add(column());

    let _root = prepare(&target);

    let row = single_row(&target);
    let cells = get_cells_presenter(&row);
    assert_eq!(cells.children().count(), 3);
    for child in cells.children().snapshot().iter() {
        assert_eq!(child.get_type(), TableViewCell::TYPE);
    }

    // Cells must also be part of the row's logical tree.
    assert!(children_of(&cells) == logical_children(&row));
}

#[test]
fn column_table_view_is_set_when_added_and_unset_when_removed() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    let column = star_column();
    target.columns().add(column.clone());

    let _root = prepare(&target);

    assert!(column.table_view() == Some(target.clone()));

    target.columns().remove(&column);

    assert!(column.table_view().is_none());
}

#[test]
fn column_logical_parent_is_set_when_added_and_unset_when_removed() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    let column = column();
    target.columns().add(column.clone());

    let _root = prepare(&target);

    assert!(column.parent() == Some(target.clone().upcast::<StyledElement>()));

    target.columns().remove(&column);

    assert!(column.parent().is_none());
}

#[test]
fn column_receives_styles_from_table_view() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    let style = Style::with_setters(
        Selectors::of_type::<TableViewColumn>().class("right-align"),
        [Setter::new(TableViewColumn::horizontal_content_alignment_property(), HorizontalAlignment::Right)],
    );
    target.styles().add(style);

    let column = column();
    column.classes().add("right-align");
    target.columns().add(column.clone());

    let _root = prepare(&target);

    assert_eq!(column.horizontal_content_alignment(), HorizontalAlignment::Right);
}

#[test]
#[should_panic(expected = "is already attached to a TableView.")]
fn column_cannot_belong_to_two_table_views() {
    let _app = start();

    let column = star_column();

    let first = create_target(strs(&["Foo"]));
    first.columns().add(column.clone());
    let _first_root = prepare(&first);

    assert!(column.table_view() == Some(first.clone()));

    let second = create_target(strs(&["Bar"]));
    second.columns().add(column.clone());

    let _second_root = prepare(&second);
}

#[test]
fn adding_column_adds_cell_to_realized_rows_and_headers() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    target.columns().add(star_column());
    target.columns().add(star_column());

    let _root = prepare(&target);

    let row = single_row(&target);
    let cells_presenter = get_cells_presenter(&row);
    assert_eq!(cells_presenter.children().count(), 2);

    let headers_presenter = get_column_headers_presenter(&target);
    assert_eq!(headers_presenter.children().count(), 2);

    target.columns().add(star_column());

    assert_eq!(cells_presenter.children().count(), 3);
    assert_eq!(headers_presenter.children().count(), 3);

    // The newly added cell must also be part of the row's logical tree.
    assert!(children_of(&cells_presenter) == logical_children(&row));
}

#[test]
fn adding_hidden_column_updates_cells_and_headers() {
    for column_index in [0usize, 1] {
        let _app = start();
        let target = create_target(strs(&["Foo"]));
        target.styles().add(Style::with_setters(
            Selectors::of_type::<TableViewColumn>().class("hidden"),
            [Setter::new(TableViewColumn::is_visible_property(), false)],
        ));
        target.columns().add(column());
        let _root = prepare(&target);
        let column = column();
        column.classes().add("hidden");

        target.columns().insert(column_index, column);
        layout(&target);

        let row = single_row(&target);
        assert!(!get_cells_presenter(&row).children().get(column_index).is_visible());
        assert!(!get_column_headers_presenter(&target).children().get(column_index).is_visible());
        assert!(get_cells_presenter(&row).children().get(1 - column_index).is_visible());
        assert!(get_column_headers_presenter(&target).children().get(1 - column_index).is_visible());
    }
}

#[test]
fn removing_column_removes_cell_from_realized_rows_and_headers() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    target.columns().add(star_column());
    target.columns().add(star_column());
    target.columns().add(star_column());

    let _root = prepare(&target);

    let row = single_row(&target);
    let cells_presenter = get_cells_presenter(&row);
    assert_eq!(cells_presenter.children().count(), 3);

    let headers_presenter = get_column_headers_presenter(&target);
    assert_eq!(headers_presenter.children().count(), 3);

    let removed_cell = cell(&cells_presenter, 2);

    target.columns().remove_at(2);

    assert_eq!(cells_presenter.children().count(), 2);
    assert_eq!(headers_presenter.children().count(), 2);

    // The removed cell must also be detached from the row's logical tree,
    // while the remaining cells stay in it.
    let logical_children = logical_children(&row);
    assert!(!logical_children.contains(&removed_cell.upcast::<StyledElement>()));
    assert!(children_of(&cells_presenter) == logical_children);
}

#[test]
fn changing_column_width_does_not_recreate_cells() {
    let _app = start();

    let column = star_column();
    let target = create_target(strs(&["Foo"]));
    target.columns().add(column.clone());
    target.columns().add(star_column());

    let _root = prepare_with(&target, 200.0, 200.0);

    let presenter = get_cells_presenter(&single_row(&target));
    let cells_before = children_of(&presenter);

    column.set_width(GridLength::new(2.0, GridUnitType::Star));
    layout(&target);

    let cells_after = children_of(&presenter);
    assert!(cells_before == cells_after);
}

#[test]
fn changing_column_width_invalidates_row_presenter_measure() {
    let _app = start();

    let column = star_column();
    let target = create_target(strs(&["Foo"]));
    target.columns().add(column.clone());
    target.columns().add(star_column());

    let _root = prepare_with(&target, 200.0, 200.0);

    let presenter = get_cells_presenter(&single_row(&target));
    assert!(presenter.is_measure_valid());

    column.set_width(GridLength::new(2.0, GridUnitType::Star));

    assert!(!presenter.is_measure_valid());
}

#[test]
fn changing_column_width_updates_actual_width_after_layout() {
    let _app = start();

    let column = star_column();
    let target = create_target(strs(&["Foo"]));
    target.columns().add(column.clone());
    target.columns().add(star_column());

    let _root = prepare_with(&target, 200.0, 200.0);

    assert_eq!(target.columns().get(0).actual_width(), 100.0);
    assert_eq!(target.columns().get(1).actual_width(), 100.0);

    column.set_width(GridLength::new(3.0, GridUnitType::Star));
    layout(&target);

    assert_eq!(target.columns().get(0).actual_width(), 150.0);
    assert_eq!(target.columns().get(1).actual_width(), 50.0);
}

#[test]
fn hiding_column_closes_gap_without_recreating_controls() {
    let _app = start();
    let column = TableViewColumn::new();
    column.set_width(GridLength::from_pixels(80.0));
    let target = create_target(strs(&["Foo"]));
    target.columns().add(column.clone());
    target.columns().add(TableViewColumn::new());
    let _root = prepare_with(&target, 200.0, 200.0);
    let row = single_row(&target);
    let cells = get_cells_presenter(&row);
    let headers = get_column_headers_presenter(&target);
    let original_cells = children_of(&cells);
    let original_headers = children_of(&headers);

    column.set_is_visible(false);
    layout(&target);

    assert!(!cells.children().get(0).is_visible());
    assert!(!headers.children().get(0).is_visible());
    assert_eq!(cells.children().get(1).bounds().x, 0.0);
    assert_eq!(headers.children().get(1).bounds().x, 0.0);
    assert_eq!(target.columns().get(1).actual_width(), 200.0);
    assert_eq!(column.actual_width(), 0.0);
    assert_eq!(column.width(), GridLength::from_pixels(80.0));

    column.set_is_visible(true);
    layout(&target);

    assert!(cells.children().get(0).is_visible());
    assert!(headers.children().get(0).is_visible());
    assert_eq!(cells.children().get(1).bounds().x, 80.0);
    assert_eq!(headers.children().get(1).bounds().x, 80.0);
    assert!(original_cells == children_of(&cells));
    assert!(original_headers == children_of(&headers));
}

#[test]
fn hidden_columns_do_not_contribute_to_row_size() {
    let _app = start();
    let target = create_target(strs(&["Foo"]));
    let column = TableViewColumn::new();
    column.set_is_visible(false);
    target.columns().add(column.clone());
    target.columns().add(TableViewColumn::new());
    let _root = prepare(&target);
    let row = single_row(&target);
    let cells = get_cells_presenter(&row);
    cells.children().get(0).set_height(80.0);
    cells.children().get(1).set_height(20.0);
    layout(&target);

    assert!(!cells.children().get(0).is_visible());
    assert!(!get_column_headers_presenter(&target).children().get(0).is_visible());
    assert_eq!(cells.desired_size().height, 20.0);

    column.set_is_visible(true);
    layout(&target);

    assert_eq!(cells.desired_size().height, 80.0);

    column.set_is_visible(false);
    target.columns().get(1).set_is_visible(false);
    layout(&target);

    assert_eq!(cells.desired_size(), Size::default());

    column.set_is_visible(true);
    layout(&target);

    assert_eq!(cells.desired_size().height, 80.0);
}

#[test]
fn replacing_columns_collection_updates_realized_rows_and_headers() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    target.columns().add(star_column());
    target.columns().add(star_column());

    let _root = prepare(&target);

    let row = single_row(&target);
    let cells_presenter = get_cells_presenter(&row);
    assert_eq!(cells_presenter.children().count(), 2);

    let headers_presenter = get_column_headers_presenter(&target);
    assert_eq!(headers_presenter.children().count(), 2);

    let old_cells = children_of(&cells_presenter);

    target.set_columns(Some(FerroList::from_items([star_column(), star_column(), star_column()])));

    assert_eq!(cells_presenter.children().count(), 3);
    assert_eq!(headers_presenter.children().count(), 3);

    // The new cells must be in the row's logical tree, and the old ones
    // gone.
    let logical_children = logical_children(&row);
    assert!(children_of(&cells_presenter) == logical_children);
    for cell in &old_cells {
        assert!(!logical_children.contains(cell));
    }
}

#[test]
fn cell_content_defaults_to_row_item_when_no_template_or_binding() {
    let _app = start();

    let item = Person::new("Alice", None);
    let target = create_target(people(&[&item]));
    target.columns().add(star_column());
    target.columns().add(star_column());

    let _root = prepare(&target);

    let row = single_row(&target);
    let first_cell = cell(&get_cells_presenter(&row), 0);

    assert!(reference_equals(&first_cell.content(), &Some(item.clone() as BoxedValue)));
}

#[test]
fn cell_uses_column_binding() {
    let _app = start();

    let target = create_target(people(&[&Person::new("Alice", None), &Person::new("Bob", None)]));
    let column = star_column();
    column.set_binding(binding("Name"));
    target.columns().add(column);
    target.columns().add(star_column());

    let _root = prepare(&target);

    let rows = rows(&target);
    let first_cell = cell(&get_cells_presenter(&rows[0]), 0);
    let second_cell = cell(&get_cells_presenter(&rows[1]), 0);

    assert_eq!(content_string(&first_cell).as_deref(), Some("Alice"));
    assert_eq!(content_string(&second_cell).as_deref(), Some("Bob"));
}

#[test]
fn hidden_column_preserves_width_and_visibility_bindings() {
    let _app = start();
    let model = Border::new();
    model.set_is_visible(false);
    model.set_tag(Some(Rc::new(GridLength::from_pixels(80.0)) as BoxedValue));
    let target = create_target(strs(&["Foo"]));
    target.set_data_context(Some(Control::boxed(model.clone())));
    let column = TableViewColumn::new();
    column.bind_binding(TableViewColumn::width_property().as_property(), &ReflectionBinding::new("Tag"));
    column.bind_binding(TableViewColumn::is_visible_property().as_property(), &ReflectionBinding::new("IsVisible"));
    target.columns().add(column.clone());
    target.columns().add(TableViewColumn::new());
    let _root = prepare_with(&target, 200.0, 200.0);
    assert!(!column.is_visible());
    assert_eq!(target.columns().get(1).actual_width(), 200.0);

    model.set_tag(Some(Rc::new(GridLength::from_pixels(120.0)) as BoxedValue));
    model.set_is_visible(true);
    layout(&target);

    assert!(column.is_visible());
    assert_eq!(column.actual_width(), 120.0);
    assert_eq!(target.columns().get(1).actual_width(), 80.0);

    model.set_is_visible(false);
    layout(&target);

    assert!(!column.is_visible());
    assert_eq!(target.columns().get(1).actual_width(), 200.0);
}

#[test]
fn cell_uses_column_cell_template() {
    let _app = start();

    let template = text_block_template();
    let item = Person::new("Alice", None);
    let target = create_target(people(&[&item]));
    let column = star_column();
    column.set_cell_template(Some(template.clone()));
    target.columns().add(column);
    target.columns().add(star_column());

    let _root = prepare(&target);

    let row = single_row(&target);
    let first_cell = cell(&get_cells_presenter(&row), 0);

    assert!(first_cell.content_template() == Some(template));
    assert!(reference_equals(&first_cell.content(), &Some(item.clone() as BoxedValue)));
}

#[test]
fn cell_uses_column_cell_theme() {
    let _app = start();

    let cell_theme = ControlTheme::with_target_type(TableViewCell::TYPE);
    let target = create_target(strs(&["Foo"]));
    let column = star_column();
    column.set_cell_theme(cell_theme.clone());
    target.columns().add(column);
    target.columns().add(star_column());

    let _root = prepare(&target);

    let row = single_row(&target);
    let first_cell = cell(&get_cells_presenter(&row), 0);
    let second_cell = cell(&get_cells_presenter(&row), 1);

    assert!(first_cell.theme() == Some(cell_theme));
    assert!(second_cell.theme().is_none());
}

#[test]
fn cell_column_property_is_set_to_owning_column() {
    let _app = start();

    let column0 = star_column();
    let column1 = star_column();
    let target = create_target(strs(&["Foo"]));
    target.columns().add(column0.clone());
    target.columns().add(column1.clone());

    let _root = prepare(&target);

    let row = single_row(&target);
    let first_cell = cell(&get_cells_presenter(&row), 0);
    let second_cell = cell(&get_cells_presenter(&row), 1);

    assert!(first_cell.column() == Some(column0));
    assert!(second_cell.column() == Some(column1));
}

#[test]
fn cell_uses_column_horizontal_content_alignment() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    let column = star_column();
    column.set_horizontal_content_alignment(HorizontalAlignment::Right);
    target.columns().add(column);
    target.columns().add(star_column());

    let _root = prepare(&target);

    let row = single_row(&target);
    let first_cell = cell(&get_cells_presenter(&row), 0);
    let second_cell = cell(&get_cells_presenter(&row), 1);

    assert_eq!(first_cell.horizontal_content_alignment(), HorizontalAlignment::Right);
    assert_eq!(second_cell.horizontal_content_alignment(), HorizontalAlignment::Left);
}

#[test]
fn changing_column_properties_updates_existing_headers_and_cells() {
    let _app = start();

    let cell_theme_a = ControlTheme::with_target_type(TableViewCell::TYPE);
    let cell_theme_b = ControlTheme::with_target_type(TableViewCell::TYPE);
    let header_theme_a = ControlTheme::with_target_type(TableViewColumnHeader::TYPE);
    let header_theme_b = ControlTheme::with_target_type(TableViewColumnHeader::TYPE);
    let header_template_a = text_block_template();
    let header_template_b = text_block_template();

    let item = Person::new("Alice", Some("Ally"));
    let column = star_column();
    column.set_cell_theme(cell_theme_a.clone());
    column.set_horizontal_content_alignment(HorizontalAlignment::Left);
    column.set_binding(binding("Name"));
    column.set_header(crate::test_support::boxed_str("H1"));
    column.set_header_theme(header_theme_a.clone());
    column.set_header_template(Some(header_template_a.clone()));
    let target = create_target(people(&[&item]));
    target.columns().add(column.clone());

    let _root = prepare(&target);

    let row = single_row(&target);
    let cell = cell(&get_cells_presenter(&row), 0);
    let header = header(&get_column_headers_presenter(&target), 0);

    assert!(cell.theme() == Some(cell_theme_a));
    assert_eq!(cell.horizontal_content_alignment(), HorizontalAlignment::Left);
    assert!(cell.content_template().is_none());
    assert_eq!(content_string(&cell).as_deref(), Some("Alice"));

    assert!(header.theme() == Some(header_theme_a));
    assert_eq!(header.horizontal_content_alignment(), HorizontalAlignment::Left);
    assert!(header.content_template() == Some(header_template_a));
    assert_eq!(header.content().as_ref().and_then(string_of).as_deref(), Some("H1"));

    // Mutating the column after the cell and header were built should
    // update both. Switch the binding first to confirm it's reflected in
    // the cell content.
    column.set_cell_theme(cell_theme_b.clone());
    column.set_horizontal_content_alignment(HorizontalAlignment::Right);
    column.set_binding(binding("Nickname"));
    column.set_header(crate::test_support::boxed_str("H2"));
    column.set_header_theme(header_theme_b.clone());
    column.set_header_template(Some(header_template_b.clone()));

    assert!(cell.theme() == Some(cell_theme_b));
    assert_eq!(cell.horizontal_content_alignment(), HorizontalAlignment::Right);
    assert!(cell.content_template().is_none());
    assert_eq!(content_string(&cell).as_deref(), Some("Ally"));

    assert!(header.theme() == Some(header_theme_b));
    assert_eq!(header.horizontal_content_alignment(), HorizontalAlignment::Right);
    assert!(header.content_template() == Some(header_template_b));
    assert_eq!(header.content().as_ref().and_then(string_of).as_deref(), Some("H2"));

    // The cell template takes priority over the binding: the row item
    // flows through the template.
    let cell_template_b = text_block_template();
    column.set_cell_template(Some(cell_template_b.clone()));

    assert!(cell.content_template() == Some(cell_template_b));
    assert!(reference_equals(&cell.content(), &Some(item.clone() as BoxedValue)));
}

#[test]
fn re_templating_row_detaches_old_cells_and_rebuilds_new_cells() {
    for is_visible in [true, false] {
        let _app = start();

        let target = create_target(strs(&["Foo"]));
        let column = TableViewColumn::new();
        column.set_is_visible(is_visible);
        target.columns().add(column);
        target.columns().add(TableViewColumn::new());

        let _root = prepare(&target);

        let row = single_row(&target);
        let old_cells = children_of(&get_cells_presenter(&row));
        assert_eq!(old_cells.len(), 2);
        assert!(old_cells == logical_children(&row));

        row.set_template(Some(row_template()));
        row.apply_template();

        let new_cells_presenter = get_cells_presenter(&row);
        let new_cells = children_of(&new_cells_presenter);
        assert_eq!(new_cells.len(), 2);
        assert_eq!(new_cells_presenter.children().get(0).is_visible(), is_visible);
        assert!(new_cells_presenter.children().get(1).is_visible());

        for cell in &old_cells {
            assert!(cell.parent().is_none());
        }
        assert!(new_cells == logical_children(&row));
    }
}

#[test]
fn can_user_resize_columns_defaults_to_true() {
    let _scope = test_scope();

    assert!(TableView::new().can_user_resize_columns());
}

#[test]
fn column_can_user_effectively_resize_combines_table_view_and_column_settings() {
    let rows = [
        (true, None, true),          // inherits the table's value
        (true, Some(true), true),    // column opts in
        (true, Some(false), false),  // column opts out
        (false, None, false),        // inherits the table's value
        (false, Some(true), true),   // column opts in
        (false, Some(false), false), // column opts out
    ];

    for (can_resize_columns, can_resize, expected) in rows {
        let _app = start();

        let target = create_target(strs(&["Foo"]));
        target.set_can_user_resize_columns(can_resize_columns);
        let column = TableViewColumn::new();
        column.set_can_user_resize(can_resize);
        target.columns().add(column.clone());

        let _root = prepare(&target);

        assert_eq!(column.can_user_effectively_resize(), expected);
    }
}

#[test]
fn column_can_user_effectively_resize_updates_when_table_view_can_resize_columns_changes() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    let column = TableViewColumn::new();
    target.columns().add(column.clone());

    let _root = prepare(&target);

    assert!(column.can_user_effectively_resize());

    target.set_can_user_resize_columns(false);
    assert!(!column.can_user_effectively_resize());

    target.set_can_user_resize_columns(true);
    assert!(column.can_user_effectively_resize());
}

#[test]
fn column_can_user_effectively_resize_updates_when_column_can_resize_changes() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    target.set_can_user_resize_columns(false);
    let column = TableViewColumn::new();
    target.columns().add(column.clone());

    let _root = prepare(&target);

    assert!(!column.can_user_effectively_resize());

    column.set_can_user_resize(Some(true));
    assert!(column.can_user_effectively_resize());

    column.set_can_user_resize(None);
    assert!(!column.can_user_effectively_resize());
}

// --- Additional tests (not ports) ---

// The column headers are outside of the scroll viewer: the headers presenter arranges them
// shifted by the horizontal offset of the scrollable (`ArrangeRow(.., -offset)`).
#[test]
fn additional_column_headers_follow_the_horizontal_scroll_offset() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    for _ in 0..3 {
        let column = column();
        column.set_width(GridLength::from_pixels(150.0));
        target.columns().add(column);
    }
    let _root = prepare_with(&target, 200.0, 200.0);
    let headers = get_column_headers_presenter(&target);

    assert_eq!(headers.children().get(0).bounds().x, 0.0);
    assert_eq!(headers.children().get(1).bounds().x, 150.0);

    // The reference theme enables horizontal scrolling of the table view.
    let scroll_viewer = target.scroll().and_then(|scroll| scroll.cast::<ScrollViewer>()).expect("a scroll viewer");
    scroll_viewer.set_horizontal_scroll_bar_visibility(crate::primitives::ScrollBarVisibility::Auto);
    layout(&target);
    scroll_viewer.set_offset(ferroui_base::Vector::new(100.0, 0.0));
    // As in the reference, the presenter subscribes to the scrollable only if the table view
    // already has one when the presenter is attached to the logical tree; in a template the
    // presenter is attached before the scroll part is found, so the offset is picked up by the
    // next arrange of the presenter rather than by a notification.
    headers.invalidate_arrange();
    layout(&target);

    assert_eq!(scroll_viewer.offset().x, 100.0);
    assert_eq!(headers.children().get(0).bounds().x, -100.0);
    assert_eq!(headers.children().get(1).bounds().x, 50.0);
    assert_eq!(headers.children().get(2).bounds().x, 200.0);
}

// The columns collection assigned through the untyped property path replaces the old one: the
// old columns are released, the new ones adopted, and only the new list is observed.
#[test]
fn additional_columns_assigned_through_the_untyped_path_replace_the_subscription() {
    let _app = start();

    let target = create_target(strs(&["Foo"]));
    let old_column = star_column();
    let old_columns = target.columns();
    old_columns.add(old_column.clone());
    let _root = prepare(&target);
    assert!(old_column.table_view() == Some(target.clone()));

    let new_column = star_column();
    let new_columns: FerroList<Ref<TableViewColumn>> = FerroList::new();
    new_columns.add(new_column.clone());
    let value: Option<FerroList<Ref<TableViewColumn>>> = Some(new_columns.clone());
    target.set_value_untyped(
        TableView::columns_property().as_property(),
        &value,
        ferroui_base::data::BindingPriority::LocalValue,
    );
    layout(&target);

    assert!(target.columns() == new_columns);
    assert!(old_column.table_view().is_none());
    assert!(new_column.table_view() == Some(target.clone()));
    assert_eq!(get_column_headers_presenter(&target).children().count(), 1);

    // The old list is no longer observed, the new one is.
    let stray = star_column();
    old_columns.add(stray.clone());
    assert!(stray.table_view().is_none());
    let added = star_column();
    new_columns.add(added.clone());
    layout(&target);
    assert!(added.table_view() == Some(target.clone()));
    assert_eq!(get_column_headers_presenter(&target).children().count(), 2);
}
