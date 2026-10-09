//! The reference tests register a mocked cursor factory; the tests ported
//! here never create a cursor, so none is registered.

use crate::primitives::{AdornerLayer, Thumb, VisualLayerManager};
use crate::test_support::{test_scope, TestRoot};
use crate::templates::FuncTemplate;
use crate::{
    Decorator, Border, ColumnDefinition, ColumnDefinitions, Control, Grid, GridLength, GridResizeDirection, GridSplitter,
    GridUnitType, RowDefinition, RowDefinitions,
};
use crate::presenters::{ContentPresenter, ItemsPresenter};
use crate::primitives::TemplatedControl;
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, ITemplateOf,
};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{ItemsControl, ItemsSource, Panel, TextBlock};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{Maybe, Value};
use ferroui_base::data::model::Model;
use ferroui_base::data::{ReflectionBinding, TemplateBinding};
use ferroui_base::styling::{ControlTheme, Selectors, Setter, Style};
use ferroui_base::{BoxedValue, ObjectType};
use std::rc::Rc;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, VectorEventArgs};
use ferroui_base::media::TranslateTransform;
use ferroui_base::{Rect, Ref, Size, Vector};

fn cell(control: impl ferroui_base::IntoRef<Control>, row: Option<i32>, column: Option<i32>) -> Ref<Control> {
    let control = control.into_ref();
    if let Some(row) = row {
        Grid::set_row(&control, row);
    }
    if let Some(column) = column {
        Grid::set_column(&control, column);
    }
    control
}

fn drag(splitter: &GridSplitter, event: &'static ferroui_base::interactivity::RoutedEvent<VectorEventArgs>, vector: Vector) {
    let mut e = VectorEventArgs::new();
    e.set_routed_event(Some(event));
    e.vector = vector;
    splitter.raise_event(&e);
}

fn drag_started(splitter: &GridSplitter) {
    drag(splitter, Thumb::drag_started_event(), Vector::default());
}

fn drag_delta(splitter: &GridSplitter, vector: Vector) {
    drag(splitter, Thumb::drag_delta_event(), vector);
}

fn drag_completed(splitter: &GridSplitter) {
    drag(splitter, Thumb::drag_completed_event(), Vector::default());
}

fn key_down(splitter: &GridSplitter, key: Key) {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key = key;
    splitter.raise_event(&e);
}

fn layer_manager(child: impl ferroui_base::IntoRef<Control>) -> Ref<VisualLayerManager> {
    let manager = VisualLayerManager::new();
    manager.set_child(child.into_ref());
    manager
}

fn star(value: f64) -> GridLength {
    GridLength::new(value, GridUnitType::Star)
}

#[test]
fn detects_horizontal_orientation() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();

    let grid = Grid::new();
    grid.set_row_definitions(RowDefinitions::parse("*,Auto,*").unwrap());
    grid.set_column_definitions(ColumnDefinitions::parse("*,*").unwrap());
    grid.children().add(cell(Border::new(), Some(0), None));
    grid.children().add(cell(splitter.clone(), Some(1), None));
    grid.children().add(cell(Border::new(), Some(2), None));

    let root = TestRoot::with_child(grid);
    root.measure(Size::new(100.0, 300.0));
    root.arrange(Rect::new(0.0, 0.0, 100.0, 300.0));
    assert_eq!(GridResizeDirection::Rows, splitter.get_effective_resize_direction());
}

#[test]
fn detects_vertical_orientation() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();

    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse("*,Auto,*").unwrap());
    grid.set_row_definitions(RowDefinitions::parse("*,*").unwrap());
    grid.children().add(cell(Border::new(), None, Some(0)));
    grid.children().add(cell(splitter.clone(), None, Some(1)));
    grid.children().add(cell(Border::new(), None, Some(2)));

    let root = TestRoot::with_child(grid);
    root.measure(Size::new(100.0, 300.0));
    root.arrange(Rect::new(0.0, 0.0, 100.0, 300.0));
    assert_eq!(GridResizeDirection::Columns, splitter.get_effective_resize_direction());
}

#[test]
fn detects_with_both_auto() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();

    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse("Auto,Auto,Auto").unwrap());
    grid.set_row_definitions(RowDefinitions::parse("Auto,Auto").unwrap());
    grid.children().add(cell(Border::new(), None, Some(0)));
    grid.children().add(cell(splitter.clone(), None, Some(1)));
    grid.children().add(cell(Border::new(), None, Some(2)));

    let root = TestRoot::with_child(grid);
    root.measure(Size::new(100.0, 300.0));
    root.arrange(Rect::new(0.0, 0.0, 100.0, 300.0));
    assert_eq!(GridResizeDirection::Columns, splitter.get_effective_resize_direction());
}

#[test]
fn in_first_position_doesnt_throw_exception() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse("Auto,*,*").unwrap());
    grid.set_row_definitions(RowDefinitions::parse("*,*").unwrap());
    grid.children().add(cell(splitter.clone(), None, Some(0)));
    grid.children().add(cell(Border::new(), None, Some(1)));
    grid.children().add(cell(Border::new(), None, Some(2)));

    let root = TestRoot::with_child(grid);
    root.measure(Size::new(100.0, 300.0));
    root.arrange(Rect::new(0.0, 0.0, 100.0, 300.0));

    drag_started(&splitter);

    drag_delta(&splitter, Vector::new(100.0, 1000.0));
}

fn horizontal_stays_within_constraints(shows_preview: bool) {
    let _scope = test_scope();
    let control1 = cell(Border::new(), Some(0), None);
    let splitter = GridSplitter::new();
    Grid::set_row(&splitter, 1);
    splitter.set_shows_preview(shows_preview);
    let control2 = cell(Border::new(), Some(2), None);

    let row0 = RowDefinition::with_value(1.0, GridUnitType::Star);
    row0.set_min_height(70.0);
    row0.set_max_height(110.0);
    let row2 = RowDefinition::with_value(1.0, GridUnitType::Star);
    row2.set_min_height(10.0);
    row2.set_max_height(140.0);
    let row_definitions =
        RowDefinitions::from_items([row0.clone(), RowDefinition::with_height(GridLength::AUTO), row2.clone()]);

    let grid = Grid::new();
    grid.set_row_definitions(row_definitions);
    grid.children().add(control1);
    grid.children().add(splitter.clone());
    grid.children().add(control2);

    let root = TestRoot::with_child(layer_manager(grid));

    root.measure(Size::new(100.0, 200.0));
    root.arrange(Rect::new(0.0, 0.0, 100.0, 200.0));

    drag_started(&splitter);

    drag_delta(&splitter, Vector::new(0.0, -100.0));

    if shows_preview {
        assert_eq!(row0.height(), star(1.0));
        assert_eq!(row2.height(), star(1.0));
    } else {
        assert_eq!(row0.height(), star(70.0));
        assert_eq!(row2.height(), star(130.0));
    }

    drag_delta(&splitter, Vector::new(0.0, 100.0));

    if shows_preview {
        assert_eq!(row0.height(), star(1.0));
        assert_eq!(row2.height(), star(1.0));
    } else {
        assert_eq!(row0.height(), star(110.0));
        assert_eq!(row2.height(), star(90.0));
    }

    drag_completed(&splitter);

    assert_eq!(row0.height(), star(110.0));
    assert_eq!(row2.height(), star(90.0));
}

#[test]
fn horizontal_stays_within_constraints_without_preview() {
    horizontal_stays_within_constraints(false);
}

#[test]
fn horizontal_stays_within_constraints_with_preview() {
    horizontal_stays_within_constraints(true);
}

fn vertical_stays_within_constraints(shows_preview: bool) {
    let _scope = test_scope();
    let control1 = cell(Border::new(), None, Some(0));
    let splitter = GridSplitter::new();
    Grid::set_column(&splitter, 1);
    splitter.set_shows_preview(shows_preview);
    let control2 = cell(Border::new(), None, Some(2));

    let column0 = ColumnDefinition::with_value(1.0, GridUnitType::Star);
    column0.set_min_width(10.0);
    column0.set_max_width(190.0);
    let column2 = ColumnDefinition::with_value(1.0, GridUnitType::Star);
    column2.set_min_width(80.0);
    column2.set_max_width(120.0);
    let column_definitions = ColumnDefinitions::from_items([
        column0.clone(),
        ColumnDefinition::with_width(GridLength::AUTO),
        column2.clone(),
    ]);

    let grid = Grid::new();
    grid.set_column_definitions(column_definitions);
    grid.children().add(control1);
    grid.children().add(splitter.clone());
    grid.children().add(control2);

    let root = TestRoot::with_child(layer_manager(grid));

    root.measure(Size::new(200.0, 100.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));

    drag_started(&splitter);

    drag_delta(&splitter, Vector::new(-100.0, 0.0));

    if shows_preview {
        assert_eq!(column0.width(), star(1.0));
        assert_eq!(column2.width(), star(1.0));
    } else {
        assert_eq!(column0.width(), star(80.0));
        assert_eq!(column2.width(), star(120.0));
    }

    drag_delta(&splitter, Vector::new(100.0, 0.0));

    if shows_preview {
        assert_eq!(column0.width(), star(1.0));
        assert_eq!(column2.width(), star(1.0));
    } else {
        assert_eq!(column0.width(), star(120.0));
        assert_eq!(column2.width(), star(80.0));
    }

    drag_completed(&splitter);

    assert_eq!(column0.width(), star(120.0));
    assert_eq!(column2.width(), star(80.0));
}

#[test]
fn vertical_stays_within_constraints_without_preview() {
    vertical_stays_within_constraints(false);
}

#[test]
fn vertical_stays_within_constraints_with_preview() {
    vertical_stays_within_constraints(true);
}

fn vertical_keyboard_input_can_move_splitter(key: Key, expected_height_first: f64, expected_height_second: f64) {
    let _scope = test_scope();
    let control1 = cell(Border::new(), Some(0), None);
    let splitter = GridSplitter::new();
    Grid::set_row(&splitter, 1);
    splitter.set_keyboard_increment(10.0);
    let control2 = cell(Border::new(), Some(2), None);

    let row0 = RowDefinition::with_value(1.0, GridUnitType::Star);
    let row2 = RowDefinition::with_value(1.0, GridUnitType::Star);
    let row_definitions =
        RowDefinitions::from_items([row0.clone(), RowDefinition::with_height(GridLength::AUTO), row2.clone()]);

    let grid = Grid::new();
    grid.set_row_definitions(row_definitions);
    grid.children().add(control1);
    grid.children().add(splitter.clone());
    grid.children().add(control2);

    let root = TestRoot::with_child(grid);

    root.measure(Size::new(200.0, 200.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 200.0));

    key_down(&splitter, key);

    assert_eq!(row0.height(), star(expected_height_first));
    assert_eq!(row2.height(), star(expected_height_second));
}

#[test]
fn vertical_keyboard_input_can_move_splitter_up() {
    vertical_keyboard_input_can_move_splitter(Key::Up, 90.0, 110.0);
}

#[test]
fn vertical_keyboard_input_can_move_splitter_down() {
    vertical_keyboard_input_can_move_splitter(Key::Down, 110.0, 90.0);
}

fn three_column_grid(
    splitter: &Ref<GridSplitter>,
    middle: GridLength,
) -> (Ref<Grid>, Ref<ColumnDefinition>, Ref<ColumnDefinition>) {
    let control1 = cell(Border::new(), None, Some(0));
    Grid::set_column(splitter, 1);
    let control2 = cell(Border::new(), None, Some(2));

    let column0 = ColumnDefinition::with_value(1.0, GridUnitType::Star);
    let column2 = ColumnDefinition::with_value(1.0, GridUnitType::Star);
    let column_definitions =
        ColumnDefinitions::from_items([column0.clone(), ColumnDefinition::with_width(middle), column2.clone()]);

    let grid = Grid::new();
    grid.set_column_definitions(column_definitions);
    grid.children().add(control1);
    grid.children().add(splitter.clone());
    grid.children().add(control2);

    (grid, column0, column2)
}

fn horizontal_keyboard_input_can_move_splitter(key: Key, expected_width_first: f64, expected_width_second: f64) {
    let _scope = test_scope();
    let splitter = GridSplitter::new();
    splitter.set_keyboard_increment(10.0);
    let (grid, column0, column2) = three_column_grid(&splitter, GridLength::AUTO);

    let root = TestRoot::with_child(grid);

    root.measure(Size::new(200.0, 200.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 200.0));

    key_down(&splitter, key);

    assert_eq!(column0.width(), star(expected_width_first));
    assert_eq!(column2.width(), star(expected_width_second));
}

#[test]
fn horizontal_keyboard_input_can_move_splitter_left() {
    horizontal_keyboard_input_can_move_splitter(Key::Left, 90.0, 110.0);
}

#[test]
fn horizontal_keyboard_input_can_move_splitter_right() {
    horizontal_keyboard_input_can_move_splitter(Key::Right, 110.0, 90.0);
}

#[test]
fn pressing_escape_key_cancels_resizing() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();
    splitter.set_keyboard_increment(10.0);
    let (grid, column0, column2) = three_column_grid(&splitter, GridLength::AUTO);

    let root = TestRoot::with_child(grid);

    root.measure(Size::new(200.0, 200.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 200.0));

    drag_started(&splitter);

    drag_delta(&splitter, Vector::new(-100.0, 0.0));

    assert_eq!(column0.width(), star(0.0));
    assert_eq!(column2.width(), star(200.0));

    key_down(&splitter, Key::Escape);

    assert_eq!(column0.width(), star(1.0));
    assert_eq!(column2.width(), star(1.0));
}

/// The reference test builds this tree from markup; it is built in code
/// here.
#[test]
fn works_in_grid() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();
    splitter.set_resize_direction(GridResizeDirection::Columns);
    let (grid, column0, column2) = three_column_grid(&splitter, GridLength::from_pixels(10.0));

    let root = TestRoot::with_child(grid);
    root.measure(Size::new(200.0, 100.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));

    drag_started(&splitter);
    drag_delta(&splitter, Vector::new(-20.0, 0.0));
    drag_completed(&splitter);

    assert_ne!(column0.width(), column2.width());
}

/// The `TextItem` of the reference.
struct TextItem {
    column: i32,
    text: Option<String>,
}

ferroui_base::ferro_model!(TextItem, |b| b
    .read_only::<Value<i32>>("Column", |item| item.column)
    .read_only::<Maybe<String>>("Text", |item| item.text.clone()));

/// The `SplitterItem` of the reference.
struct SplitterItem {
    column: i32,
}

ferroui_base::ferro_model!(SplitterItem, |b| b.read_only::<Value<i32>>("Column", |item| item.column));

/// An items control with the control theme of the markup of the reference
/// tests in its resources and a grid with the columns `*,10,*` as its items
/// panel.
fn items_control_with_grid_panel() -> Ref<ItemsControl> {
    let template = FuncControlTemplate::for_type::<ItemsControl>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));
        let border = Border::new();
        for property in [
            TemplatedControl::background_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
            TemplatedControl::corner_radius_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        ] {
            border.bind_binding(property, &TemplateBinding::new(property));
        }
        border.set_child(presenter.register_in_name_scope(&**scope));
        border.upcast()
    });
    let theme = ControlTheme::with_setters(
        ItemsControl::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(template))],
    );

    let items_control = ItemsControl::new();
    items_control.resources().add_value(ResourceKey::Type(ItemsControl::TYPE), theme);
    let items_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| {
        let grid = Grid::new();
        grid.set_column_definitions(ColumnDefinitions::parse("*,10,*").unwrap());
        Some(grid.upcast::<Panel>())
    });
    items_control.set_items_panel(items_panel);
    items_control
}

/// The reference test builds this tree from markup; it is built in code
/// here.
#[test]
fn works_in_items_control_items_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let items_control = items_control_with_grid_panel();
    items_control.styles().add(Style::with_setters(
        Selectors::of_type::<ItemsControl>().child().of_type::<ContentPresenter>(),
        [Setter::new_binding_base(Grid::column_property().as_property(), ReflectionBinding::new("Column"))],
    ));
    items_control.data_templates().add(FuncDataTemplate::for_type::<TextItem>(
        |_, _| {
            let text_block = TextBlock::new();
            text_block.bind_binding(TextBlock::text_property().as_property(), &*ReflectionBinding::new("Text"));
            let border = Border::new();
            border.set_child(text_block);
            Some(border.upcast())
        },
        false,
    ));
    items_control.data_templates().add(FuncDataTemplate::for_type::<SplitterItem>(
        |_, _| {
            let splitter = GridSplitter::new();
            splitter.set_resize_direction(GridResizeDirection::Columns);
            Some(splitter.upcast())
        },
        false,
    ));
    let items: [BoxedValue; 3] = [
        Model::new_model(TextItem { column: 0, text: Some("A".to_string()) }),
        Model::new_model(SplitterItem { column: 1 }),
        Model::new_model(TextItem { column: 2, text: Some("B".to_string()) }),
    ];
    items_control.set_items_source(Some(ItemsSource::from_items(items.map(Some))));

    let root = TestRoot::with_child(&items_control);
    root.measure(Size::new(200.0, 100.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));

    let panel = items_control.items_panel_root().and_then(|panel| panel.cast::<Grid>()).expect("a grid");
    let cp = panel.children().get(1).cast::<ContentPresenter>().expect("a content presenter");
    cp.update_child();
    let splitter = cp.child().and_then(|child| child.cast::<GridSplitter>()).expect("a grid splitter");

    drag_started(&splitter);
    drag_delta(&splitter, Vector::new(-20.0, 0.0));
    drag_completed(&splitter);

    assert_ne!(panel.column_definitions().get(0).width(), panel.column_definitions().get(2).width());
}

/// The reference test builds this tree from markup; it is built in code
/// here.
#[test]
fn works_in_items_control_items() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let items_control = items_control_with_grid_panel();
    let splitter = GridSplitter::new();
    splitter.set_resize_direction(GridResizeDirection::Columns);
    items_control.items().add(Some(Control::boxed(cell(Border::new(), None, Some(0)))));
    items_control.items().add(Some(Control::boxed(cell(splitter, None, Some(1)))));
    items_control.items().add(Some(Control::boxed(cell(Border::new(), None, Some(2)))));

    let root = TestRoot::with_child(&items_control);
    root.measure(Size::new(200.0, 100.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));

    let panel = items_control.items_panel_root().and_then(|panel| panel.cast::<Grid>()).expect("a grid");
    let splitter = panel.children().get(1).cast::<GridSplitter>().expect("a grid splitter");

    drag_started(&splitter);
    drag_delta(&splitter, Vector::new(-20.0, 0.0));
    drag_completed(&splitter);

    assert_ne!(panel.column_definitions().get(0).width(), panel.column_definitions().get(2).width());
}

// Additional tests of the preview adorner, which the reference suite only
// covers through the two theories above.

fn preview_grid(splitter: &Ref<GridSplitter>) -> (Ref<TestRoot>, Ref<VisualLayerManager>, Ref<ColumnDefinition>, Ref<ColumnDefinition>) {
    splitter.set_shows_preview(true);
    let (grid, column0, column2) = three_column_grid(splitter, GridLength::from_pixels(10.0));
    let manager = layer_manager(grid);
    let root = TestRoot::with_child(manager.clone());
    root.measure(Size::new(210.0, 100.0));
    root.arrange(Rect::new(0.0, 0.0, 210.0, 100.0));
    (root, manager, column0, column2)
}

fn adorners(manager: &VisualLayerManager) -> Vec<Ref<Control>> {
    manager.adorner_layer().map(|layer| layer.children().snapshot().to_vec()).unwrap_or_default()
}

#[test]
fn preview_adorner_is_shown_while_dragging_and_removed_when_the_drag_completes() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();
    let preview = Border::new();
    splitter.set_preview_content(Some(FuncTemplate::new({
        let preview: Ref<Control> = preview.clone().upcast();
        move || preview.clone()
    })));
    let (_root, manager, column0, column2) = preview_grid(&splitter);
    assert!(adorners(&manager).is_empty());

    drag_started(&splitter);

    let shown = adorners(&manager);
    assert_eq!(1, shown.len());
    let adorner = &shown[0];
    assert_eq!(Some(splitter.clone().upcast()), AdornerLayer::get_adorned_element(adorner));
    assert!(!AdornerLayer::get_is_clip_enabled(adorner));
    let decorator = adorner.cast::<Decorator>().and_then(|adorner| adorner.child()).expect("the adorner has a decorator");
    assert_eq!(Some(preview.upcast()), decorator.cast::<Decorator>().and_then(|decorator| decorator.child()));
    let translation = decorator
        .render_transform()
        .and_then(|transform| transform.as_object().and_then(|t| t.downcast_ref::<TranslateTransform>()).map(|t| t.to_ref()))
        .expect("the decorator is translated");

    // The second drag started event the thumb sometimes raises is ignored.
    drag_started(&splitter);
    assert_eq!(1, adorners(&manager).len());

    drag_delta(&splitter, Vector::new(-20.0, 5.0));
    assert_eq!((-20.0, 0.0), (translation.x(), translation.y()));
    assert_eq!((star(1.0), star(1.0)), (column0.width(), column2.width()));

    // The preview stays within the constraints of the definitions.
    drag_delta(&splitter, Vector::new(-500.0, 0.0));
    assert_eq!(-100.0, translation.x());
    drag_delta(&splitter, Vector::new(30.0, 0.0));
    assert_eq!(30.0, translation.x());

    drag_completed(&splitter);

    assert!(adorners(&manager).is_empty());
    assert_eq!((star(130.0), star(70.0)), (column0.width(), column2.width()));
}

#[test]
fn preview_adorner_is_removed_when_the_resize_is_cancelled() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();
    let (_root, manager, column0, column2) = preview_grid(&splitter);

    drag_started(&splitter);
    drag_delta(&splitter, Vector::new(-20.0, 0.0));
    assert_eq!(1, adorners(&manager).len());

    key_down(&splitter, Key::Escape);

    assert!(adorners(&manager).is_empty());
    assert_eq!((star(1.0), star(1.0)), (column0.width(), column2.width()));

    // The drag was cancelled: completing it changes nothing.
    drag_completed(&splitter);
    assert_eq!((star(1.0), star(1.0)), (column0.width(), column2.width()));
}

#[test]
fn keyboard_does_not_show_the_preview() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();
    splitter.set_keyboard_increment(10.0);
    let (_root, manager, column0, column2) = preview_grid(&splitter);

    key_down(&splitter, Key::Left);

    assert!(adorners(&manager).is_empty());
    assert_eq!((star(90.0), star(110.0)), (column0.width(), column2.width()));
}

#[test]
fn preview_without_an_adorner_layer_does_not_resize() {
    let _scope = test_scope();
    let splitter = GridSplitter::new();
    splitter.set_shows_preview(true);
    let (grid, column0, column2) = three_column_grid(&splitter, GridLength::from_pixels(10.0));
    let root = TestRoot::with_child(grid);
    root.measure(Size::new(210.0, 100.0));
    root.arrange(Rect::new(0.0, 0.0, 210.0, 100.0));

    drag_started(&splitter);
    drag_delta(&splitter, Vector::new(-20.0, 0.0));
    drag_completed(&splitter);

    assert_eq!((star(1.0), star(1.0)), (column0.width(), column2.width()));
}
