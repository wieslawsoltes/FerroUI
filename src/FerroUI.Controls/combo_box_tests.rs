//! Port of the reference `ComboBoxTests`.
//!
//! The tests of the reference that show a window do so with the simple theme
//! applied. That theme is not available, so [`start_styled_window`] adds
//! control themes with the templates those tests depend on (of the combo
//! box, of the content controls and of the scroll viewer) to the test theme.
//!
//! Adapted:
//!
//! - `Can_Open_DropDown_After_Replacing_Application_Theme` is run once, with
//!   the test theme, instead of once for each of the two themes of the
//!   reference (which are not available).

use crate::mouse_test_helper::MouseTestHelper;
use crate::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter};
use crate::primitives::{Popup, SelectingItemsControl, TextSearch};
use crate::shapes::Rectangle;
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
    IDataTemplate, ITemplateOf,
};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot, TestScope};
use crate::testing::{add_template_theme, create_test_theme, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    items_equal, Application, AssignedBinding, Canvas, ComboBox, ComboBoxItem, ContentControl, Control,
    DataValidationErrors, Decorator, ItemsSource, Panel, ScrollViewer, StackPanel, TextBlock, TextBox, VirtualizingStackPanel,
    Window,
};
use ferroui_base::data::core::Value;
use ferroui_base::data::model::Model;
use ferroui_base::data::{
    BindingBase, BindingError, BindingErrorType, BindingMode, BindingNotification, BindingPriority, CompiledBinding,
    CompiledBindingPathBuilder, ReflectionBinding, TemplateBinding,
};
use ferroui_base::input::{
    IKeyboardDevice, IKeyboardNavigationHandler, InputElement, Key, KeyEventArgs, KeyModifiers, KeyboardDevice,
    KeyboardNavigation, KeyboardNavigationHandler, KeyboardNavigationMode, TextInputEventArgs,
};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FlowDirection, VisualBrush};
use ferroui_base::reactive::Observable;
use ferroui_base::styling::{styles_as_style, IStyle, Styles, ThemeVariant};
use ferroui_base::{ferro_model, AnyValue, BoxedValue, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;

/// The scope of a test of the reference that starts no application.
fn start() -> TestScope {
    test_scope()
}

/// A running unit test application, with the text services of the tests
/// registered over the services of the application.
struct AppScope {
    // Dropped in this order: the text services first.
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

fn start_app(services: TestServices) -> AppScope {
    let app = UnitTestApplication::start(services);
    AppScope { _text: TextTestScope::new(), _app: app }
}

/// Adds the control themes that stand in for those of the simple theme of
/// the reference: the templates of the combo box, of the content controls
/// and of the scroll viewer.
fn add_theme(styles: &Ref<Styles>) {
    add_template_theme::<ComboBox>(styles, create_template(true));
    add_template_theme::<ContentControl>(styles, content_control_template());
    add_template_theme::<ComboBoxItem>(styles, content_control_template());
    add_template_theme::<ScrollViewer>(styles, scroll_viewer_template());
}

/// Starts an application with the services of the tests that show windows.
fn start_styled_window_with(services: TestServices) -> AppScope {
    let scope = start_app(services);
    add_theme(&Application::current().expect("the application is running").styles());
    scope
}

fn start_styled_window() -> AppScope {
    start_styled_window_with(TestServices::styled_window())
}

// --- helpers ----------------------------------------------------------------

fn get_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(create_template(false))
}

/// The template of the tests. With `bind_selection_box_template` the
/// content template of the selection box is bound to the template of the
/// selection box item, as the themes of the reference do.
fn create_template(bind_selection_box_template: bool) -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ComboBox>(move |_, scope| {
        let selection_box = ContentControl::new();
        selection_box.bind_binding(
            ContentControl::content_property().as_property(),
            &TemplateBinding::new(ComboBox::selection_box_item_property().as_property()),
        );
        if bind_selection_box_template {
            selection_box.bind_binding(
                ContentControl::content_template_property().as_property(),
                &TemplateBinding::new(ComboBox::selection_box_item_template_property().as_property()),
            );
        }

        let items_presenter = ItemsPresenter::new();
        items_presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let items_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(VirtualizingStackPanel::new().upcast::<Panel>()));
        items_presenter.set_items_panel(items_panel);

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        scroll_viewer.set_content(Some(Control::boxed(items_presenter.register_in_name_scope(&**scope))));

        let popup = Popup::new();
        popup.set_name(Some("PART_Popup".to_string()));
        popup.bind_binding(
            Popup::is_open_property().as_property(),
            &TemplateBinding::new(ComboBox::is_drop_down_open_property().as_property()).with_mode(BindingMode::TwoWay),
        );
        popup.set_child(scroll_viewer.register_in_name_scope(&**scope));

        let panel = Panel::new();
        panel.set_name(Some("container".to_string()));
        panel.children().add(selection_box);
        panel.children().add(popup.register_in_name_scope(&**scope));
        let text_box = TextBox::new();
        text_box.set_name(Some("PART_EditableTextBox".to_string()));
        panel.children().add(text_box.register_in_name_scope(&**scope));
        panel.upcast()
    })
}

/// The template of a content control: a content presenter.
fn content_control_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ContentControl>(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        presenter.bind_binding(
            ContentPresenter::content_template_property().as_property(),
            &TemplateBinding::new(ContentControl::content_template_property().as_property()),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

/// The template of a scroll viewer: a scroll content presenter.
fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

/// Shows a window with `target` as its content and runs the initial layout
/// pass.
fn show_window(target: &Ref<ComboBox>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(target.clone())));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    window
}

fn combo_box_item(content: &str) -> Ref<ComboBoxItem> {
    let item = ComboBoxItem::new();
    item.set_content(boxed_str(content));
    item
}

fn binding(path: &str) -> Option<AssignedBinding> {
    let binding: Rc<dyn BindingBase> = ReflectionBinding::new(path);
    Some(AssignedBinding::new(binding))
}

/// A binding to the tag of a control.
fn tag_binding() -> Option<AssignedBinding> {
    let path = CompiledBindingPathBuilder::new().ferro_property(Control::tag_property().as_property()).build();
    let binding: Rc<dyn BindingBase> = CompiledBinding::new(path);
    Some(AssignedBinding::new(binding))
}

fn empty_binding() -> Option<AssignedBinding> {
    let binding: Rc<dyn BindingBase> = Rc::new(ReflectionBinding::default());
    Some(AssignedBinding::new(binding))
}

fn raise_key_down(target: &Control, key: Key, modifiers: KeyModifiers) {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    args.key_modifiers = modifiers;
    target.raise_event(&args);
}

fn raise_key_up(target: &Control, key: Key, modifiers: KeyModifiers) {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_up_event()));
    args.key = key;
    args.key_modifiers = modifiers;
    target.raise_event(&args);
}

fn raise_tab_key_press(target: &Control, with_shift: bool) {
    let modifiers = if with_shift { KeyModifiers::SHIFT } else { KeyModifiers::NONE };
    raise_key_down(target, Key::Tab, modifiers);
    raise_key_up(target, Key::Tab, modifiers);
}

/// A data template for any data that builds a text block tagged with the
/// data.
fn tagging_template() -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::new(
        |_| true,
        |x, _| {
            let text_block = TextBlock::new();
            text_block.set_tag(x.clone());
            Some(text_block.upcast())
        },
        false,
    ))
}

/// A data template for strings that builds a text block showing the
/// string followed by `suffix`.
fn string_template(suffix: &'static str) -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::for_type::<String>(
        move |x, _| {
            let text_block = TextBlock::new();
            text_block.set_text(Some(&format!("{x}{suffix}")));
            Some(text_block.upcast())
        },
        false,
    ))
}

/// The text block that the selection box control shows.
fn selection_box_text_block(target: &ComboBox) -> Ref<TextBlock> {
    get_selection_box_control(target)
        .presenter()
        .and_then(|presenter| presenter.child())
        .and_then(|child| child.cast::<TextBlock>())
        .expect("the selection box control shows a text block")
}

fn assert_selection_box_item_is_visual_brush_of(target: &ComboBox, item: &Ref<Control>) {
    let rectangle = selection_box_rectangle(target).expect("the selection box item is a rectangle");
    let fill = rectangle.fill().expect("the rectangle has a fill");
    let visual_brush =
        fill.as_object().and_then(|object| object.downcast_ref::<VisualBrush>()).expect("the fill is a visual brush");
    assert!(visual_brush.visual() == Some(item.clone().upcast()));
}

/// The selection box item, when it is a rectangle.
fn selection_box_rectangle(target: &ComboBox) -> Option<Ref<Rectangle>> {
    target
        .get_direct_value(ComboBox::selection_box_item_property())
        .as_ref()
        .and_then(Control::from_boxed)
        .and_then(|control| control.cast::<Rectangle>())
}

fn get_selection_box_control(target: &ComboBox) -> Ref<ContentControl> {
    let selection_box_item_template = target.selection_box_item_template();
    let selection_box_control = target
        .find_descendant_of_type_where::<ContentControl>(false, |c| c.content_template() == selection_box_item_template);
    selection_box_control.expect("the selection box control was not found")
}

/// Whether an untyped value is the given control.
fn is_control(value: &Option<BoxedValue>, control: &Ref<Control>) -> bool {
    value.as_ref().and_then(Control::from_boxed).is_some_and(|value| value == *control)
}

struct Item {
    value: String,
    display: String,
}

ferro_model!(Item, |b| b
    .read_only::<Value<String>>("Value", |x| x.value.clone())
    .read_only::<Value<String>>("Display", |x| x.display.clone()));

impl Item {
    fn new(value: &str, display: &str) -> Rc<Item> {
        Model::new_model(Item { value: value.to_string(), display: display.to_string() })
    }
}

fn item(value: &Rc<Item>) -> Option<BoxedValue> {
    Some(value.clone() as BoxedValue)
}

fn item_source(items: &[Rc<Item>]) -> Option<ItemsSource> {
    Some(ItemsSource::from_items(items.iter().map(item)))
}

/// Whether an untyped value is the given model instance.
fn is_item(value: &Option<BoxedValue>, expected: &Rc<Item>) -> bool {
    value.as_ref().is_some_and(|value| {
        let value: &dyn AnyValue = &**value;
        value.downcast_ref::<Item>().is_some_and(|x| std::ptr::eq(x, &**expected))
    })
}

// --- tests ------------------------------------------------------------------

/// The reference runs the test with each of its two themes, which are not
/// available: it is run with the test theme and the control themes of
/// [`add_theme`] instead.
#[test]
fn can_open_drop_down_after_replacing_application_theme() {
    fn create_theme() -> Rc<dyn IStyle> {
        let styles = Styles::new();
        styles.add(create_test_theme());
        add_theme(&styles);
        styles_as_style(&styles)
    }

    let _app = start_app(TestServices::styled_window().with_theme(create_theme));
    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar"])));
    target.set_selected_index(0);
    let window = show_window(&target);

    for i in 0..3 {
        target.set_is_drop_down_open(true);
        window.layout_manager().execute_layout_pass();
        target.set_is_drop_down_open(false);
        window.layout_manager().execute_layout_pass();

        let application = Application::current().unwrap();
        application.styles().clear();
        application.styles().add(create_theme());
        application.set_requested_theme_variant(Some(if i % 2 == 0 { ThemeVariant::dark() } else { ThemeVariant::light() }));
        window.layout_manager().execute_layout_pass();

        target.set_is_drop_down_open(true);
        window.layout_manager().execute_layout_pass();
        assert!(target.container_from_index(0).is_some());
        assert_eq!(0, target.selected_index());
        target.set_is_drop_down_open(false);
        window.layout_manager().execute_layout_pass();
    }
    window.close();
}

#[test]
fn clicking_on_control_toggles_is_drop_down_open() {
    let _scope = start();
    let helper = MouseTestHelper::new();
    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar"])));

    helper.down(&target);
    helper.up(&target);
    assert!(target.is_drop_down_open());
    assert!(target.classes().contains(ComboBox::PC_DROPDOWN_OPEN));

    helper.down(&target);
    helper.up(&target);

    assert!(!target.is_drop_down_open());
    assert!(!target.classes().contains(ComboBox::PC_DROPDOWN_OPEN));
}

#[test]
fn clicking_on_control_pseudo_class() {
    let _scope = start();
    let helper = MouseTestHelper::new();
    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar"])));

    helper.down(&target);
    assert!(target.classes().contains(ComboBox::PC_PRESSED));
    helper.up(&target);
    assert!(!target.classes().contains(ComboBox::PC_PRESSED));
    assert!(target.classes().contains(ComboBox::PC_DROPDOWN_OPEN));

    helper.down(&target);
    assert!(!target.classes().contains(ComboBox::PC_PRESSED));
    helper.up(&target);
    assert!(!target.classes().contains(ComboBox::PC_PRESSED));

    assert!(!target.is_drop_down_open());
    assert!(!target.classes().contains(ComboBox::PC_DROPDOWN_OPEN));
}

#[test]
fn wrap_selection_should_work() {
    let _app = start_app(TestServices::real_focus());
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(combo_box_item("bla"))));
    target.items().add(Some(Control::boxed(combo_box_item("dd"))));
    let disabled = combo_box_item("sdf");
    disabled.set_is_enabled(false);
    target.items().add(Some(Control::boxed(disabled)));
    target.set_template(get_template());
    target.set_wrap_selection(true);

    let _root = TestRoot::with_child(&target);
    target.apply_template();
    target.presenter().unwrap().apply_template();
    target.focus();
    assert_eq!(target.selected_index(), -1);
    assert!(target.is_focused());
    raise_key_down(&target, Key::Up, KeyModifiers::NONE);
    assert_eq!(target.selected_index(), 1);
    raise_key_down(&target, Key::Down, KeyModifiers::NONE);
    assert_eq!(target.selected_index(), 0);
}

#[test]
fn focuses_next_item_on_key_down() {
    let _app = start_app(TestServices::real_focus());
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(combo_box_item("bla"))));
    let disabled = combo_box_item("dd");
    disabled.set_is_enabled(false);
    target.items().add(Some(Control::boxed(disabled)));
    target.items().add(Some(Control::boxed(combo_box_item("sdf"))));
    target.set_template(get_template());

    let _root = TestRoot::with_child(&target);
    target.apply_template();
    target.presenter().unwrap().apply_template();
    target.focus();
    assert_eq!(target.selected_index(), -1);
    assert!(target.is_focused());
    raise_key_down(&target, Key::Down, KeyModifiers::NONE);
    assert_eq!(target.selected_index(), 0);
    raise_key_down(&target, Key::Down, KeyModifiers::NONE);
    assert_eq!(target.selected_index(), 2);
}

#[test]
fn selection_box_item_is_rectangle_with_visual_brush_when_selection_is_control() {
    let _scope = start();
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(Canvas::new())));
    target.set_selected_index(0);
    let _root = TestRoot::with_child(&target);

    let rectangle = selection_box_rectangle(&target);
    assert!(rectangle.is_some());

    let fill = rectangle.unwrap().fill();
    let brush = fill.as_ref().and_then(|fill| fill.as_object()).and_then(|object| object.downcast_ref::<VisualBrush>());
    assert!(brush.is_some());
    let item = target.items().get_at(0).as_ref().and_then(Control::from_boxed).unwrap();
    assert!(brush.unwrap().visual() == Some(item.upcast()));
}

#[test]
fn selection_box_item_rectangle_is_removed_from_logical_tree() {
    let _scope = start();
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(Canvas::new())));
    target.set_selected_index(0);
    target.set_template(get_template());

    let root = TestRoot::new();
    root.set_child(target.clone());
    target.apply_template();
    target.presenter().unwrap().apply_template();

    let rectangle = selection_box_rectangle(&target);
    assert!(rectangle.is_some());
    let rectangle = rectangle.unwrap();
    assert!(target.is_attached_to_logical_tree());
    assert!(rectangle.is_attached_to_logical_tree());

    rectangle.detached_from_logical_tree(|_| {});

    root.set_child(None);

    assert!(!target.is_attached_to_logical_tree());
    assert!(!rectangle.is_attached_to_logical_tree());
}

/// The reference test scrolls the first scroll viewer among the visual
/// descendants of the combo box, which, with the popup closed, is the
/// scroll viewer of the template of the text box. The test theme has no
/// template for the text box, so the scroll viewer of the popup itself is
/// scrolled to the top.
#[test]
fn reopening_drop_down_focuses_selected_item_after_scrolled_to_top() {
    let _app = start_styled_window_with(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>))
            .with_keyboard_navigation(|| Some(KeyboardNavigationHandler::new() as Rc<dyn IKeyboardNavigationHandler>)),
    );

    let target = ComboBox::new();
    target.set_template(get_template());

    for i in 0..100 {
        target.items().add(Some(Control::boxed(combo_box_item(&format!("Item {i}")))));
    }

    let selected_item =
        target.items().get_at(60).as_ref().and_then(Control::from_boxed).and_then(|item| item.cast::<ComboBoxItem>());
    assert!(selected_item.is_some());
    let selected_item = selected_item.unwrap();

    let window = show_window(&target);
    target.apply_template();
    target.presenter().unwrap().apply_template();

    let popup = target.get_visual_descendants().find_map(|x| x.cast::<Popup>()).unwrap();
    let scroll_viewer = popup.child().and_then(|child| child.cast::<ScrollViewer>());
    assert!(scroll_viewer.is_some());
    let scroll_viewer = scroll_viewer.unwrap();

    target.set_selected_item(Some(Control::boxed(selected_item.clone())));
    target.focus();
    target.set_is_drop_down_open(true);
    window.layout_manager().execute_layout_pass();

    scroll_viewer.scroll_to_home();
    target.set_is_drop_down_open(false);
    window.layout_manager().execute_layout_pass();

    target.set_is_drop_down_open(true);
    window.layout_manager().execute_layout_pass();

    assert!(selected_item.is_focused() && selected_item.is_visible());
}


#[test]
fn detaching_closed_combo_box_keeps_current_focus() {
    let _app = start_app(TestServices::real_focus());
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(Canvas::new())));
    target.set_selected_index(0);
    target.set_template(get_template());

    let other = Control::new();
    other.set_focusable(true);

    let panel = StackPanel::new();
    panel.children().add(target.clone());
    panel.children().add(other.clone());
    let _root = TestRoot::with_child(&panel);

    target.apply_template();
    target.presenter().unwrap().apply_template();

    other.focus();

    assert!(other.is_focused());

    panel.children().remove(target.clone());

    assert!(other.is_focused());
}

#[test]
fn text_search_should_have_expected_selected_index() {
    const LONG: [&str; 36] = [
        "0 item", "1 item", "2 item", "3 item", "4 item", "5 item", "6 item", "7 item", "8 item", "9 item", "A item",
        "B item", "C item", "D item", "E item", "F item", "G item", "H item", "I item", "J item", "K item", "L item",
        "M item", "N item", "O item", "P item", "Q item", "R item", "S item", "T item", "U item", "V item", "W item",
        "X item", "Y item", "Z item",
    ];
    let cases: [(i32, i32, &str, &[&str]); 4] = [
        (-1, 2, "c", &["A item", "B item", "C item"]),
        (0, 1, "b", &["A item", "B item", "C item"]),
        (2, 2, "x", &["A item", "B item", "C item"]),
        (0, 34, "y", &LONG),
    ];

    for (initial_selected_index, expected_selected_index, search_term, contents) in cases {
        test_text_search(
            initial_selected_index,
            expected_selected_index,
            search_term,
            |_| {},
            contents.iter().map(|content| Some(Control::boxed(combo_box_item(content)))).collect(),
        );
    }
}

const TEXT_SEARCH_CASES: [(i32, i32, &str, [&str; 3], [&str; 3]); 2] = [
    (-1, 1, "c", ["A item", "B item", "C item"], ["B search", "C search", "A search"]),
    (0, 2, "baz", ["A item", "B item", "C item"], ["foo", "bar", "baz"]),
];

#[test]
fn text_search_with_text_search_text_should_have_expected_selected_index() {
    for (initial_selected_index, expected_selected_index, search_term, contents, search_texts) in TEXT_SEARCH_CASES {
        assert_eq!(contents.len(), search_texts.len());

        test_text_search(
            initial_selected_index,
            expected_selected_index,
            search_term,
            |_| {},
            contents
                .iter()
                .zip(search_texts)
                .map(|(item, search_text)| {
                    let combo_box_item = combo_box_item(item);
                    TextSearch::set_text(&combo_box_item, Some(search_text.to_string()));
                    Some(Control::boxed(combo_box_item))
                })
                .collect(),
        );
    }
}

#[test]
fn text_search_with_display_member_binding_should_have_expected_selected_index() {
    for (initial_selected_index, expected_selected_index, search_term, values, displays) in TEXT_SEARCH_CASES {
        assert_eq!(values.len(), displays.len());

        test_text_search(
            initial_selected_index,
            expected_selected_index,
            search_term,
            |combo_box| combo_box.set_display_member_binding(binding("Display")),
            values.iter().zip(displays).map(|(value, display)| item(&Item::new(value, display))).collect(),
        );
    }
}

#[test]
fn text_search_with_text_search_binding_should_have_expected_selected_index() {
    for (initial_selected_index, expected_selected_index, search_term, values, displays) in TEXT_SEARCH_CASES {
        assert_eq!(values.len(), displays.len());

        test_text_search(
            initial_selected_index,
            expected_selected_index,
            search_term,
            |combo_box| TextSearch::set_text_binding(combo_box, binding("Display")),
            values.iter().zip(displays).map(|(value, display)| item(&Item::new(value, display))).collect(),
        );
    }
}

fn test_text_search(
    initial_selected_index: i32,
    expected_selected_index: i32,
    search_term: &str,
    configure_combo_box: impl Fn(&ComboBox),
    items_source: Vec<Option<BoxedValue>>,
) {
    let _app = start_styled_window();
    let target = ComboBox::new();
    target.set_template(get_template());
    target.set_items_source(Some(ItemsSource::from_items(items_source)));

    configure_combo_box(&target);

    let root = TestRoot::with_child(&target);
    root.set_client_size(Size::new(500.0, 500.0));

    root.execute_initial_layout_pass();
    target.set_selected_index(initial_selected_index);

    let mut args = TextInputEventArgs::new();
    args.text = Some(search_term.to_string());
    args.set_routed_event(Some(InputElement::text_input_event()));

    target.raise_event(&args);

    assert_eq!(expected_selected_index, target.selected_index());
}

#[test]
fn selected_item_validation() {
    let _app = start_app(TestServices::mock_threading_interface());
    let target = ComboBox::new();
    target.set_template(get_template());

    target.apply_template();
    target.presenter().unwrap().apply_template();

    let exception = BindingError::message("failed validation");
    let text_observable = Observable::single_value(Rc::new(BindingNotification::with_error(
        exception.clone(),
        BindingErrorType::DataValidationError,
    )) as BoxedValue);
    target.bind_property_untyped(
        SelectingItemsControl::selected_item_property().as_property(),
        text_observable,
        BindingPriority::LocalValue,
    );

    assert!(DataValidationErrors::get_has_errors(&target));
    assert_errors_are(&target, &exception);
}

#[test]
fn text_validation() {
    let _app = start_app(TestServices::mock_threading_interface());
    let target = ComboBox::new();
    target.set_template(get_template());

    target.apply_template();
    target.presenter().unwrap().apply_template();

    let exception = BindingError::message("failed validation");
    let text_observable = Observable::single_value(Rc::new(BindingNotification::with_error(
        exception.clone(),
        BindingErrorType::DataValidationError,
    )) as BoxedValue);
    target.bind_property_untyped(ComboBox::text_property().as_property(), text_observable, BindingPriority::LocalValue);

    assert!(DataValidationErrors::get_has_errors(&target));
    assert_errors_are(&target, &exception);
}

/// Asserts that the data validation errors of the control are exactly
/// `exception`.
fn assert_errors_are(target: &Control, exception: &BindingError) {
    let errors = DataValidationErrors::get_errors(target).map(|errors| errors.to_vec()).unwrap_or_default();
    assert_eq!(1, errors.len());
    let error = errors[0].clone();
    let error: &dyn AnyValue = &*error;
    assert!(error.downcast_ref::<BindingError>() == Some(exception));
}

#[test]
fn close_window_on_alt_f4_when_combo_box_is_focus() {
    let _app = start_styled_window();

    let window = Window::new();

    let weak = window.downgrade();
    window.add_handler(InputElement::key_down_event(), move |_, e: &KeyEventArgs| {
        if !e.handled() && e.key_modifiers.contains(KeyModifiers::ALT) && e.key == Key::F4 {
            e.set_handled(true);
            if let Some(window) = weak.upgrade() {
                window.close();
            }
        }
    });

    let count = Rc::new(Cell::new(0));

    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(Canvas::new())));
    target.set_selected_index(0);
    target.set_template(get_template());

    window.set_content(Some(Control::boxed(target.clone())));

    let counter = count.clone();
    window.closing(move |_| counter.set(counter.get() + 1));

    window.show();

    target.focus();

    let helper = MouseTestHelper::new();
    helper.down(&target);
    helper.up(&target);
    assert!(target.is_drop_down_open());

    raise_key_down(&target, Key::F4, KeyModifiers::ALT);

    assert_eq!(1, count.get());
}

#[test]
fn flow_direction_of_rectangle_content_should_be_left_to_right() {
    let _app = start_styled_window();

    let target = ComboBox::new();
    target.set_flow_direction(FlowDirection::RightToLeft);
    let item = ComboBoxItem::new();
    item.set_content(Some(Control::boxed(Control::new())));
    target.items().add(Some(Control::boxed(item)));
    target.set_template(get_template());

    let _root = TestRoot::with_child(&target);
    target.apply_template();
    target.set_selected_index(0);

    let rectangle = selection_box_rectangle(&target);
    assert!(rectangle.is_some());
    assert_eq!(FlowDirection::LeftToRight, rectangle.unwrap().flow_direction());
}

#[test]
fn flow_direction_of_rectangle_content_updated_after_invalidate_mirror_transform() {
    let _app = start_styled_window();

    let parent_content = Decorator::new();
    parent_content.set_child(Control::new());
    let target = ComboBox::new();
    let item = ComboBoxItem::new();
    item.set_content(Some(Control::boxed(parent_content.child().unwrap())));
    target.items().add(Some(Control::boxed(item)));
    target.set_template(get_template());

    let _root = TestRoot::with_child(&target);
    target.apply_template();
    target.set_selected_index(0);

    let rectangle = selection_box_rectangle(&target);
    assert!(rectangle.is_some());
    let rectangle = rectangle.unwrap();
    assert_eq!(FlowDirection::LeftToRight, rectangle.flow_direction());

    parent_content.set_flow_direction(FlowDirection::RightToLeft);
    target.set_flow_direction(FlowDirection::RightToLeft);

    assert_eq!(FlowDirection::RightToLeft, rectangle.flow_direction());
}

#[test]
fn flow_direction_of_rectangle_content_updated_after_open_popup() {
    let _app = start_styled_window();

    let parent_content = Decorator::new();
    parent_content.set_child(Control::new());
    let target = ComboBox::new();
    target.set_flow_direction(FlowDirection::RightToLeft);
    let item = ComboBoxItem::new();
    item.set_content(Some(Control::boxed(parent_content.child().unwrap())));
    // Ugly hack, so we can "attach" the same child to the two different
    // trees.
    item.set_template(None);
    target.items().add(Some(Control::boxed(item)));
    target.set_template(get_template());

    let _root = TestRoot::with_child(&target);
    target.apply_template();
    target.set_selected_index(0);

    let rectangle = selection_box_rectangle(&target);
    assert!(rectangle.is_some());
    let rectangle = rectangle.unwrap();
    assert_eq!(FlowDirection::LeftToRight, rectangle.flow_direction());

    parent_content.set_flow_direction(FlowDirection::RightToLeft);

    let popup = target.get_visual_descendants().find_map(|x| x.cast::<Popup>()).unwrap();
    popup.set_placement_target(Window::new());
    popup.open();

    assert_eq!(FlowDirection::RightToLeft, rectangle.flow_direction());
}

#[test]
fn selection_box_item_template_overrides_item_template() {
    let _scope = start();
    let item_template = string_template("!");
    let selection_box_item_template = string_template("");
    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo"])));
    target.set_selection_box_item_template(selection_box_item_template.clone());
    target.set_item_template(item_template);

    assert!(selection_box_item_template == target.selection_box_item_template());
}

#[test]
fn selection_box_item_template_inherits_from_item_template_when_not_set() {
    let _scope = start();
    let item_template = string_template("!");
    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo"])));
    target.set_item_template(item_template.clone());

    assert!(item_template == target.selection_box_item_template());
}

#[test]
fn selection_box_item_template_overrides_item_template_after_item_template_changed() {
    let _scope = start();
    let item_template = string_template("!");
    let selection_box_item_template = string_template("");
    let item_template2 = string_template("?");
    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo"])));
    target.set_selection_box_item_template(selection_box_item_template.clone());
    target.set_item_template(item_template);

    assert!(selection_box_item_template == target.selection_box_item_template());

    target.set_item_template(item_template2);

    assert!(selection_box_item_template == target.selection_box_item_template());
}

#[test]
fn selection_box_item_template_inherits_from_item_template_when_item_template_changed() {
    let _scope = start();
    let item_template = string_template("!");
    let item_template2 = string_template("?");
    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo"])));
    target.set_item_template(item_template.clone());

    assert!(item_template == target.selection_box_item_template());

    target.set_item_template(item_template2.clone());
    target.set_selection_box_item_template(None);

    assert!(item_template2 == target.selection_box_item_template());
}

#[test]
fn display_member_binding_is_not_applied_to_selection_box_item_without_selection() {
    let _scope = start();
    let target = ComboBox::new();
    target.set_display_member_binding(empty_binding());
    target.set_items_source(Some(ItemsSource::from_strs(["foo", "bar"])));

    target.set_selected_item(None);
    assert!(target.selection_box_item().is_none());

    target.set_selected_item(boxed_str("foo"));
    assert!(target.selection_box_item().is_some());

    target.set_selected_item(None);
    assert!(target.selection_box_item().is_none());
}

#[test]
fn item_template_is_applied_to_control_item_in_drop_down() {
    let _app = start_styled_window();

    let item: Ref<Control> = Canvas::new().upcast();
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(item.clone())));
    target.set_item_template(tagging_template());
    target.set_selected_index(0);

    let window = show_window(&target);

    target.set_is_drop_down_open(true);
    window.layout_manager().execute_layout_pass();

    let container = target.container_from_index(0).and_then(|container| container.cast::<ComboBoxItem>()).unwrap();
    let text_block =
        container.presenter().and_then(|presenter| presenter.child()).and_then(|child| child.cast::<TextBlock>());
    assert!(is_control(&text_block.unwrap().tag(), &item));
}

#[test]
fn display_member_binding_is_applied_to_control_item_in_drop_down() {
    let _app = start_styled_window();

    let item = Canvas::new();
    item.set_tag(boxed_str("foo"));
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(item)));
    target.set_display_member_binding(tag_binding());
    target.set_selected_index(0);

    let window = show_window(&target);

    target.set_is_drop_down_open(true);
    window.layout_manager().execute_layout_pass();

    let container = target.container_from_index(0).and_then(|container| container.cast::<ComboBoxItem>()).unwrap();
    let text_block =
        container.presenter().and_then(|presenter| presenter.child()).and_then(|child| child.cast::<TextBlock>());
    assert_eq!(Some("foo".to_string()), text_block.unwrap().text());
}

#[test]
fn item_template_is_applied_to_control_item_in_selection_box() {
    let _app = start_styled_window();

    let item: Ref<Control> = Canvas::new().upcast();
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(item.clone())));
    target.set_item_template(tagging_template());
    target.set_selected_index(0);

    let _window = show_window(&target);

    let selection_box_control = get_selection_box_control(&target);
    let text_block = selection_box_control
        .presenter()
        .and_then(|presenter| presenter.child())
        .and_then(|child| child.cast::<TextBlock>());
    assert!(is_control(&text_block.unwrap().tag(), &item));
}

#[test]
fn display_member_binding_is_applied_to_control_item_in_selection_box() {
    let _app = start_styled_window();

    let item = Canvas::new();
    item.set_tag(boxed_str("foo"));

    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(item)));
    target.set_display_member_binding(tag_binding());
    target.set_selected_index(0);

    let _window = show_window(&target);

    let text_block = selection_box_text_block(&target);
    assert_eq!(Some("foo".to_string()), text_block.text());
}

#[test]
fn selection_box_item_template_is_applied_to_control_item_in_selection_box() {
    let _app = start_styled_window();

    let item: Ref<Control> = Canvas::new().upcast();
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(item.clone())));
    target.set_selection_box_item_template(tagging_template());
    target.set_selected_index(0);

    let _window = show_window(&target);

    let selection_box_control = get_selection_box_control(&target);
    let text_block = selection_box_control
        .presenter()
        .and_then(|presenter| presenter.child())
        .and_then(|child| child.cast::<TextBlock>());
    assert!(is_control(&text_block.unwrap().tag(), &item));
}

#[test]
fn item_template_is_applied_to_control_item_in_selection_box_when_changed() {
    let _app = start_styled_window();

    let item: Ref<Control> = Canvas::new().upcast();
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(item.clone())));
    target.set_selected_index(0);

    let window = show_window(&target);

    assert_selection_box_item_is_visual_brush_of(&target, &item);

    target.set_item_template(tagging_template());
    window.layout_manager().execute_layout_pass();

    let text_block = selection_box_text_block(&target);
    assert!(is_control(&text_block.tag(), &item));

    target.set_item_template(None);
    window.layout_manager().execute_layout_pass();

    assert_selection_box_item_is_visual_brush_of(&target, &item);
}

#[test]
fn display_member_binding_is_applied_to_control_item_in_selection_box_when_changed() {
    let _app = start_styled_window();

    let item: Ref<Control> = Canvas::new().upcast();
    item.set_tag(boxed_str("foo"));
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(item.clone())));
    target.set_selected_index(0);

    let window = show_window(&target);

    assert_selection_box_item_is_visual_brush_of(&target, &item);

    target.set_display_member_binding(tag_binding());
    window.layout_manager().execute_layout_pass();

    let text_block = selection_box_text_block(&target);
    assert_eq!(Some("foo".to_string()), text_block.text());

    target.set_display_member_binding(None);
    window.layout_manager().execute_layout_pass();

    assert_selection_box_item_is_visual_brush_of(&target, &item);
}

#[test]
fn selection_box_item_template_is_applied_to_control_item_in_selection_box_when_changed() {
    let _app = start_styled_window();

    let item: Ref<Control> = Canvas::new().upcast();
    let target = ComboBox::new();
    target.items().add(Some(Control::boxed(item.clone())));
    target.set_selected_index(0);

    let window = show_window(&target);

    assert_selection_box_item_is_visual_brush_of(&target, &item);

    target.set_selection_box_item_template(tagging_template());
    window.layout_manager().execute_layout_pass();

    let text_block = selection_box_text_block(&target);
    assert!(is_control(&text_block.tag(), &item));

    target.set_selection_box_item_template(None);
    window.layout_manager().execute_layout_pass();

    assert_selection_box_item_is_visual_brush_of(&target, &item);
}

#[test]
fn when_editable_input_text_matches_an_item_it_is_selected() {
    let _scope = start();
    let target = ComboBox::new();
    target.set_display_member_binding(empty_binding());
    target.set_is_editable(true);
    target.set_items_source(Some(ItemsSource::from_strs(["foo", "bar"])));

    target.set_selected_item(None);
    assert!(target.selected_item().is_none());

    target.set_text(Some("foo".to_string()));
    assert!(target.selected_item().is_some());
    assert!(items_equal(&target.selected_item(), &boxed_str("foo")));
}

#[test]
fn when_editable_text_search_text_binding_is_prioritised_over_display_member() {
    let _scope = start();
    let items = [Item::new("Value 1", "Display 1"), Item::new("Value 2", "Display 2")];
    let target = ComboBox::new();
    target.set_display_member_binding(binding("Display"));
    target.set_is_editable(true);
    target.set_items_source(item_source(&items));
    TextSearch::set_text_binding(&target, binding("Value"));

    target.set_selected_item(None);
    assert!(target.selected_item().is_none());

    target.set_text(Some("Value 1".to_string()));
    assert!(target.selected_item().is_some());
    assert!(is_item(&target.selected_item(), &items[0]));
}

#[test]
fn when_items_source_changes_it_selects_an_item_by_text() {
    let _scope = start();
    let items = [Item::new("Value 1", "Display 1"), Item::new("Value 2", "Display 2")];
    let items2 = [Item::new("Value 1", "Display 3"), Item::new("Value 2", "Display 4")];
    let target = ComboBox::new();
    target.set_display_member_binding(binding("Display"));
    target.set_is_editable(true);
    target.set_items_source(item_source(&items));
    TextSearch::set_text_binding(&target, binding("Value"));

    target.set_selected_item(None);
    assert!(target.selected_item().is_none());

    target.set_text(Some("Value 1".to_string()));
    assert!(target.selected_item().is_some());
    assert!(is_item(&target.selected_item(), &items[0]));

    target.set_items_source(item_source(&items2));
    assert!(target.selected_item().is_some());
    assert!(is_item(&target.selected_item(), &items2[0]));
    assert_eq!(target.text().as_deref(), Some("Value 1"));
}

#[test]
fn when_tabbing_out_with_dropdown_open_it_closes() {
    let _app = start_app(TestServices::real_focus());
    let helper = MouseTestHelper::new();

    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar"])));
    let next_control = ComboBox::new();
    next_control.set_items_source(Some(ItemsSource::from_strs(["Baz"])));

    let container = StackPanel::new();
    container.children().add(target.clone());
    container.children().add(next_control.clone());
    let root = TestRoot::with_child(&container);
    let keyboard_nav_handler = KeyboardNavigationHandler::new();
    keyboard_nav_handler.set_owner(&root.clone().upcast());

    target.focus();
    helper.down(&target);
    helper.up(&target);
    assert!(target.is_focused());
    assert!(target.is_drop_down_open());

    raise_tab_key_press(&target, false);

    assert!(!target.is_focused());
    assert!(next_control.is_focused());
    assert!(!target.is_drop_down_open());
}

#[test]
fn when_editable_and_item_selected_via_text_then_focus_swaps_via_tab_swapping_back_should_focus_text_box() {
    let _app = start_app(TestServices::real_focus());

    let items = [Item::new("Value 1", "Display 1"), Item::new("Value 2", "Display 2")];
    let target = ComboBox::new();
    target.set_display_member_binding(binding("Display"));
    target.set_is_editable(true);
    target.set_is_tab_stop(false);
    target.set_items_source(item_source(&items));
    target.set_template(get_template());
    TextSearch::set_text_binding(&target, binding("Value"));
    KeyboardNavigation::set_tab_navigation(&target, KeyboardNavigationMode::Local);

    let previous_control = ComboBox::new();
    previous_control.set_items_source(Some(ItemsSource::from_strs(["Baz"])));

    let container = StackPanel::new();
    container.children().add(previous_control.clone());
    container.children().add(target.clone());
    let root = TestRoot::with_child(&container);
    let keyboard_nav_handler = KeyboardNavigationHandler::new();
    keyboard_nav_handler.set_owner(&root.clone().upcast());

    target.apply_template();
    target.presenter().unwrap().apply_template();

    let container_panel = target
        .get_template_descendants()
        .into_iter()
        .filter_map(|x| x.cast::<Panel>())
        .find(|x| x.name().as_deref() == Some("container"));
    let editable_text_box = container_panel.as_ref().and_then(|panel| {
        panel
            .get_visual_descendants()
            .filter_map(|x| x.cast::<TextBox>())
            .find(|x| x.name().as_deref() == Some("PART_EditableTextBox"))
    });
    let popup = container_panel.as_ref().and_then(|panel| {
        panel.get_visual_descendants().filter_map(|x| x.cast::<Popup>()).find(|x| x.name().as_deref() == Some("PART_Popup"))
    });
    let popup_scroll_viewer = popup.and_then(|popup| popup.child()).and_then(|child| child.cast::<ScrollViewer>());
    let scroll_viewer_items_presenter = popup_scroll_viewer
        .and_then(|scroll_viewer| scroll_viewer.content())
        .as_ref()
        .and_then(Control::from_boxed)
        .and_then(|content| content.cast::<ItemsPresenter>());
    let popup_virtualizing_stack_panel = scroll_viewer_items_presenter
        .as_ref()
        .and_then(|presenter| presenter.get_visual_descendants().find_map(|x| x.cast::<VirtualizingStackPanel>()));

    assert!(editable_text_box.is_some());
    assert!(scroll_viewer_items_presenter.is_some());
    assert!(popup_virtualizing_stack_panel.is_some());
    let editable_text_box = editable_text_box.unwrap();
    let scroll_viewer_items_presenter = scroll_viewer_items_presenter.unwrap();
    let popup_virtualizing_stack_panel = popup_virtualizing_stack_panel.unwrap();

    // Force the popup to render the combo box item(s) as they are what get
    // set as "focused" if this test fails.
    popup_virtualizing_stack_panel.measure(Size::INFINITY);

    target.focus();
    assert!(editable_text_box.is_focused());

    target.set_text(Some("Value 1".to_string()));
    assert!(is_item(&target.selected_item(), &items[0]));
    let item1 = scroll_viewer_items_presenter.container_from_index(0);
    assert!(item1.is_some_and(|item| item.is::<ComboBoxItem>()));

    raise_tab_key_press(&target, true);

    assert!(!target.is_focused());
    assert!(previous_control.is_focused());

    raise_tab_key_press(&previous_control, false);

    let focused = root.focus_manager().get_focused_element();
    assert!(focused.is_some_and(|focused| focused.ptr_eq(&editable_text_box)));
}

// --- additional tests (not from the reference) --------------------------------

#[test]
#[should_panic(expected = "Could not find control 'PART_Popup'.")]
fn applying_a_template_without_popup_part_panics() {
    let _scope = start();
    let target = ComboBox::new();
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<ComboBox>(|_, _| Panel::new().upcast());
    target.set_template(Some(template));

    target.apply_template();
}

#[test]
fn opening_and_closing_the_popup_raises_drop_down_events() {
    let _app = start_styled_window();
    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar"])));
    target.set_template(get_template());

    let opened = Rc::new(Cell::new(0));
    let closed = Rc::new(Cell::new(0));
    let counter = opened.clone();
    target.drop_down_opened(move || counter.set(counter.get() + 1));
    let counter = closed.clone();
    target.drop_down_closed(move || counter.set(counter.get() + 1));

    let _window = show_window(&target);

    target.set_is_drop_down_open(true);
    assert_eq!((1, 0), (opened.get(), closed.get()));

    // Hiding the combo box closes the drop-down.
    target.set_is_visible(false);
    assert!(!target.is_drop_down_open());
    assert_eq!((1, 1), (opened.get(), closed.get()));
}

#[test]
fn releasing_the_pointer_on_an_item_of_the_open_drop_down_selects_it_and_closes_the_popup() {
    let _app = start_styled_window();
    let helper = MouseTestHelper::new();
    let target = ComboBox::new();
    for content in ["Foo", "Bar"] {
        let item = combo_box_item(content);
        item.set_width(100.0);
        item.set_height(20.0);
        target.items().add(Some(Control::boxed(item)));
    }

    let window = show_window(&target);

    target.set_is_drop_down_open(true);
    window.layout_manager().execute_layout_pass();

    let container = target.container_from_index(1).unwrap();
    assert_eq!(Some("Bar".to_string()), container.cast::<ComboBoxItem>().unwrap().content().and_then(|x| string_of(&x)));

    helper.down(&container);
    assert!(target.is_drop_down_open());
    assert_eq!(-1, target.selected_index());

    helper.up(&container);
    assert_eq!(1, target.selected_index());
    assert!(!target.is_drop_down_open());
    assert!(!target.classes().contains(ComboBox::PC_PRESSED));
}
