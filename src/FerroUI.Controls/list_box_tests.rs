//! Tests of the list box.
//!
//! Observable collections of the reference tests are notifying lists, plain
//! arrays are non-notifying items sources and view models are model types.
//!
//! The reference hosts two of the tests in a window with the simple theme,
//! which is not available: those tests give the list box the control
//! template that the other tests use.
//!
//! `LayoutManager_Should_Measure_Arrange_All` reads the private measure and
//! arrange queues of the layout manager through reflection in the reference;
//! here it reads their counts through the test accessors of the layout
//! manager.
//!
//! Not ported:
//!
//! - `Should_Not_Handle_Space_When_TextBox_Inside_ListBoxItem`: needs the
//!   text box (TEXT-SEAM).

use crate::mouse_test_helper::MouseTestHelper;
use crate::presenters::{ItemsPresenter, ScrollContentPresenter};
use crate::primitives::{ScrollBar, SelectedItemsList, SelectingItemsControl};
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
    IDataTemplate, ITemplateOf,
};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot, TestScope};
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    items_equal, Border, Button, Canvas, ContentControl, Control, DataValidationErrors, Dock, DockPanel, IItemsList,
    ItemsChangedEventArgs, ItemsChangedHandler, ItemsControl, ItemsSource, ListBox, ListBoxItem, Panel, ScrollViewer,
    SelectionMode, SizeToContent, TextBlock, VirtualizingStackPanel, Window,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::{Value, ValueTypes};
use ferroui_base::data::model::Model;
use ferroui_base::data::{
    BindingError, BindingErrorType, BindingMode, BindingNotification, BindingPriority, IndexerBinding, ReflectionBinding,
};
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::{
    IKeyboardDevice, IKeyboardNavigationHandler, IPointer, InputElement, Key, KeyEventArgs, KeyModifiers,
    KeyboardDevice, KeyboardNavigationHandler, MouseButton, Pointer, PointerPointProperties, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::layout::{ILayoutManager, Orientation, VerticalAlignment};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::DefaultPlatformSettings;
use ferroui_base::reactive::Observable;
use ferroui_base::styling::{ControlTheme, Setter};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_model, AnyValue, BoxedValue, FerroLocator, Point, Ref, Size, Vector, Visual};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(PartialEq)]
struct Item {
    #[allow(dead_code)]
    value: String,
}

impl Item {
    fn new(value: &str) -> Rc<Self> {
        Rc::new(Self { value: value.to_string() })
    }
}

struct ItemViewModel {
    caption: String,
}

ferro_model!(ItemViewModel, |b| b.read_only::<Value<String>>("Caption", |vm| vm.caption.clone()));

impl ItemViewModel {
    fn new(caption: &str) -> Rc<Self> {
        Model::new_model(Self { caption: caption.to_string() })
    }
}

/// The data context of
/// `initial_binding_of_selected_items_should_not_cause_write_to_selected_items`.
struct SelectedItemsViewModel {
    items: ItemsSource,
    selected_items: SelectedItemsList,
}

ferro_model!(SelectedItemsViewModel, |b| b
    .read_only::<Value<ItemsSource>>("Items", |vm| vm.items.clone())
    .read_only::<Value<SelectedItemsList>>("SelectedItems", |vm| vm.selected_items.clone()));

/// A list that only ever signals a reset.
struct ResettingCollection {
    items: RefCell<Vec<String>>,
    collection_changed: HandlerList<ItemsChangedHandler>,
}

impl ResettingCollection {
    fn new(item_count: usize) -> Rc<Self> {
        Rc::new(Self {
            items: RefCell::new((0..item_count).map(|x| format!("Item{x}")).collect()),
            collection_changed: HandlerList::new(),
        })
    }

    fn reverse(&self) {
        self.items.borrow_mut().reverse();
        let e = ItemsChangedEventArgs::RESET;
        for (_, handler) in self.collection_changed.snapshot().iter() {
            handler(&e);
        }
    }
}

impl IItemsList for ResettingCollection {
    fn count(&self) -> usize {
        self.items.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        boxed_str(&self.items.borrow()[index])
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        Some(self.collection_changed.add(handler))
    }

    fn remove_collection_changed(&self, token: u64) {
        self.collection_changed.remove(token);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A list that creates its items when they are first read.
struct DataVirtualizingList {
    inner: RefCell<Vec<Option<String>>>,
}

impl DataVirtualizingList {
    fn new() -> Rc<Self> {
        Rc::new(Self { inner: RefCell::new(vec![None; 100]) })
    }

    fn get_realized_items(&self) -> Vec<String> {
        self.inner.borrow().iter().flatten().cloned().collect()
    }
}

impl IItemsList for DataVirtualizingList {
    fn count(&self) -> usize {
        self.inner.borrow().len()
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        let value = format!("Item{index}");
        self.inner.borrow_mut()[index] = Some(value.clone());
        boxed_str(&value)
    }

    fn is_notifying(&self) -> bool {
        false
    }

    fn add_collection_changed(&self, _handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        None
    }

    fn remove_collection_changed(&self, _token: u64) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn start() -> TestScope {
    let scope = test_scope();
    FerroLocator::current_mutable()
        .bind::<PlatformHotkeyConfiguration>()
        .to_constant(Rc::new(PlatformHotkeyConfiguration::default()));
    scope
}

/// A running unit test application with the services of the tests that show
/// windows, and the text services of the tests registered over them.
struct StyledWindowScope {
    // Dropped in this order: the text services first.
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

fn start_styled_window() -> StyledWindowScope {
    let app = UnitTestApplication::start(TestServices::styled_window());
    StyledWindowScope { _text: TextTestScope::new(), _app: app }
}

/// A visible window of 100 by 100.
fn visible_window() -> Ref<Window> {
    let wnd = Window::new();
    wnd.set_width(100.0);
    wnd.set_height(100.0);
    wnd.set_is_visible(true);
    wnd
}

/// A test scope with a keyboard device, for the tests that move the focus.
fn start_with_focus() -> TestScope {
    let scope = start();
    FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(KeyboardDevice::new());
    scope
}

fn list_box_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ListBox>(|parent, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &IndexerBinding::new(parent.clone().upcast(), property, BindingMode::OneWay));

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        scroll_viewer.set_template(Some(scroll_viewer_template()));
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));
        scroll_viewer.register_in_name_scope(&**scope).upcast()
    })
}

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));

        let scroll_bar = ScrollBar::new();
        scroll_bar.set_name(Some("verticalScrollBar".to_string()));
        scroll_bar.set_orientation(Orientation::Vertical);

        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.children().add(scroll_bar);
        panel.upcast()
    })
}

fn templated_list_box() -> Ref<ListBox> {
    let target = ListBox::new();
    target.set_template(Some(list_box_template()));
    target
}

/// A test root with the platform settings that the test application of the
/// reference supplies.
fn test_root() -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_platform_settings(Some(Rc::new(DefaultPlatformSettings::new())));
    root
}

fn prepare(target: &Ref<ListBox>) -> Ref<TestRoot> {
    target.set_height(100.0);
    target.set_width(100.0);
    let root = test_root();
    root.set_child(target.clone());
    root.execute_initial_layout_pass();
    root
}

fn layout(c: &Control) {
    if let Some(layout_manager) = c.get_layout_manager() {
        layout_manager.execute_layout_pass();
    }
}

fn strs(values: &[&str]) -> Option<ItemsSource> {
    Some(ItemsSource::from_strs(values.iter().copied()))
}

/// The items "Item 0", "Item 1", ... as a plain array.
fn numbered(count: usize) -> Option<ItemsSource> {
    Some(ItemsSource::from_items((0..count).map(|x| boxed_str(&format!("Item {x}")))))
}

/// A notifying list of the items "Item 0", "Item 1", ...
fn numbered_list(count: usize) -> Rc<FerroList<String>> {
    Rc::new(FerroList::from_items((0..count).map(|x| format!("Item {x}"))))
}

/// A data template for strings that creates a text block of the given size.
fn text_block_template(width: Option<f64>, height: f64) -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::for_type::<String>(
        move |_, _| {
            let text_block = TextBlock::new();
            if let Some(width) = width {
                text_block.set_width(width);
            }
            text_block.set_height(height);
            Some(text_block.upcast())
        },
        false,
    ))
}

fn canvas_template(height: Option<f64>) -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::for_type::<String>(
        move |_, _| {
            let canvas = Canvas::new();
            if let Some(height) = height {
                canvas.set_height(height);
            }
            Some(canvas.upcast())
        },
        false,
    ))
}

fn border_template() -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let border = Border::new();
            border.set_height(10.0);
            Some(border.upcast())
        },
        false,
    ))
}

fn panel_of(target: &ListBox) -> Ref<Panel> {
    target.presenter().expect("no presenter").panel().expect("no panel")
}

fn panel_child(target: &ListBox, index: usize) -> Ref<ListBoxItem> {
    panel_of(target).children().get(index).cast::<ListBoxItem>().expect("the child is not a list box item")
}

fn logical_list_box_items(target: &ListBox) -> Vec<Ref<ListBoxItem>> {
    target.logical_children().to_vec().into_iter().filter_map(|x| x.cast::<ListBoxItem>()).collect()
}

fn realized_data_contexts(target: &ListBox) -> Vec<String> {
    target
        .get_realized_containers()
        .into_iter()
        .map(|x| {
            let container = x.cast::<ListBoxItem>().expect("the container is not a list box item");
            container.data_context().as_ref().and_then(string_of).expect("the data context is not a string")
        })
        .collect()
}

fn data_context_string(control: Option<Ref<Control>>) -> Option<String> {
    control.expect("no container").data_context().as_ref().and_then(string_of)
}

fn scroll_element(target: &ListBox) -> Ref<InputElement> {
    target.scroll().expect("the list box has no scrollable")
}

fn scroll_offset(target: &ListBox) -> Vector {
    scroll_element(target).as_scrollable().unwrap().offset()
}

fn set_scroll_offset(target: &ListBox, value: Vector) {
    scroll_element(target).as_scrollable().unwrap().set_offset(value)
}

fn raise_pressed_event(mouse: &MouseTestHelper, item: &Ref<ListBoxItem>, mouse_button: MouseButton) {
    mouse.click_with(item, item, mouse_button, None, KeyModifiers::NONE);
}

fn raise_key_event(target: &Interactive, key: Key, input_modifiers: KeyModifiers) {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key_modifiers = input_modifiers;
    e.key = key;
    target.raise_event(&e);
}

fn is_focused(container: Option<Ref<Control>>) -> bool {
    container.expect("no container").is_focused()
}

fn focused_element(root: &TestRoot) -> Option<Ref<InputElement>> {
    root.focus_manager().get_focused_element()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn should_use_item_template_to_create_item_content() {
    let _app = start();
    let target = templated_list_box();
    target.set_items_source(strs(&["Foo"]));
    target.set_item_template(canvas_template(None));

    let _root = prepare(&target);

    let container = panel_child(&target, 0);
    assert_eq!(container.presenter().unwrap().child().unwrap().get_type(), Canvas::TYPE);
}

#[test]
fn list_box_should_find_items_presenter_in_scroll_viewer() {
    let _app = start();
    let target = templated_list_box();

    let _root = prepare(&target);

    assert_eq!(target.presenter().unwrap().get_type(), ItemsPresenter::TYPE);
}

#[test]
fn list_box_should_find_scrollviewer_in_template() {
    let _app = start();
    let target = templated_list_box();

    let viewer: Rc<RefCell<Option<Ref<ScrollViewer>>>> = Rc::new(RefCell::new(None));

    let handler_viewer = viewer.clone();
    let weak = target.downgrade();
    target.template_applied(move |_, _| {
        let target = weak.upgrade().unwrap();
        *handler_viewer.borrow_mut() = target.scroll().and_then(|scroll| scroll.cast::<ScrollViewer>());
    });

    let _root = prepare(&target);

    assert!(viewer.borrow().is_some());
}

#[test]
fn list_box_item_containers_should_be_generated() {
    let _app = start();
    let items = ["Foo", "Bar", "Baz "];
    let target = templated_list_box();
    target.set_items_source(strs(&items));

    let _root = prepare(&target);

    let text: Vec<String> = panel_of(&target)
        .children()
        .to_vec()
        .into_iter()
        .filter_map(|x| x.cast::<ListBoxItem>())
        .filter_map(|x| x.presenter().unwrap().child())
        .filter_map(|x| x.cast::<TextBlock>())
        .map(|x| x.text().unwrap())
        .collect();

    assert_eq!(items.to_vec(), text);
}

#[test]
fn container_should_have_theme_set_to_item_container_theme() {
    let _app = start();
    let theme = ControlTheme::with_setters(ListBoxItem::TYPE, []);
    let target = templated_list_box();
    target.set_items_source(strs(&["Foo", "Bar", "Baz "]));
    target.set_item_container_theme(Some(theme.clone()));

    let _root = prepare(&target);

    let container = panel_child(&target, 0);

    assert_eq!(container.theme(), Some(theme));
}

#[test]
fn inline_item_should_have_theme_set_to_item_container_theme() {
    let _app = start();
    let theme = ControlTheme::with_setters(ListBoxItem::TYPE, []);
    let target = templated_list_box();
    target.items().add(Some(Control::boxed(ListBoxItem::new())));
    target.set_item_container_theme(Some(theme.clone()));

    let _root = prepare(&target);

    let container = panel_child(&target, 0);

    assert_eq!(container.theme(), Some(theme));
}

#[test]
fn logical_children_should_be_set_for_data_template_generated_items() {
    let _app = start();
    let target = templated_list_box();
    target.set_items_source(strs(&["Foo", "Bar", "Baz "]));

    let _root = prepare(&target);

    assert_eq!(3, target.logical_children().count());

    for child in target.logical_children().to_vec() {
        assert_eq!(child.get_type(), ListBoxItem::TYPE);
    }
}

#[test]
fn data_contexts_should_be_correctly_set() {
    let _app = start();
    let text_block = TextBlock::new();
    text_block.set_text(Some("Baz"));
    let list_box_item = ListBoxItem::new();
    list_box_item.set_content(boxed_str("Qux"));
    let item: BoxedValue = Item::new("Bar");
    let items: Vec<Option<BoxedValue>> = vec![
        boxed_str("Foo"),
        Some(item),
        Some(Control::boxed(text_block)),
        Some(Control::boxed(list_box_item)),
    ];

    let target = templated_list_box();
    target.set_data_context(boxed_str("Base"));
    target.set_item_template(Some(FuncDataTemplate::new(
        |data| {
            data.is_some_and(|data| {
                let value: &dyn AnyValue = &**data;
                value.downcast_ref::<Item>().is_some()
            })
        },
        |x, _| {
            let button = Button::new();
            button.set_content(x.clone());
            Some(button.upcast())
        },
        false,
    )));
    target.set_items_source(Some(ItemsSource::from_items(items.iter().cloned())));

    let _root = prepare(&target);

    let data_contexts: Vec<Option<BoxedValue>> =
        panel_of(&target).children().to_vec().into_iter().map(|x| x.data_context()).collect();

    let expected = [items[0].clone(), items[1].clone(), boxed_str("Base"), boxed_str("Base")];
    assert_eq!(expected.len(), data_contexts.len());
    for (expected, actual) in expected.iter().zip(data_contexts.iter()) {
        assert!(items_equal(expected, actual), "the data contexts differ");
    }
}

#[test]
fn selection_should_be_cleared_on_recycled_items() {
    let _app = start();
    let target = templated_list_box();
    target.set_items_source(numbered(20));
    target.set_item_template(text_block_template(None, 10.0));
    target.set_selected_index(0);

    let _root = prepare(&target);

    // Make sure we're virtualized and first item is selected.
    assert_eq!(10, panel_of(&target).children().count());
    assert!(panel_child(&target, 0).is_selected());

    // The selected item must not be the anchor, otherwise it won't get
    // recycled.
    target.selection().set_anchor_index(-1);

    // Scroll down a page.
    set_scroll_offset(&target, Vector::new(0.0, 10.0));
    layout(&target);

    // Make sure recycled item isn't now selected.
    assert!(!panel_child(&target, 0).is_selected());
}

#[test]
fn scroll_viewer_should_have_correct_extent_and_viewport() {
    let _app = start();
    let target = templated_list_box();
    target.set_items_source(numbered(20));
    target.set_item_template(text_block_template(Some(20.0), 10.0));
    target.set_selected_index(0);

    let _root = prepare(&target);

    let scroll = scroll_element(&target);
    assert_eq!(Size::new(100.0, 200.0), scroll.as_scrollable().unwrap().extent());
    assert_eq!(Size::new(100.0, 100.0), scroll.as_scrollable().unwrap().viewport());
}

#[test]
fn containers_correct_after_clear_add_remove() {
    let _app = start();
    // Issue #1936
    let items = numbered_list(11);
    let target = templated_list_box();
    target.set_items_source(Some(items.clone().into()));
    target.set_item_template(text_block_template(Some(20.0), 10.0));
    target.set_selected_index(0);

    let _root = prepare(&target);

    items.clear();
    items.add_range((0..11).map(|x| format!("Item {x}")));
    layout(&target);

    items.remove(&"Item 2".to_string());
    layout(&target);

    let mut actual: Vec<String> = target
        .get_realized_containers()
        .into_iter()
        .map(|x| x.cast::<ListBoxItem>().unwrap().content().as_ref().and_then(string_of).unwrap())
        .collect();
    let mut expected = items.to_vec();
    expected.sort();
    actual.sort();
    assert_eq!(expected, actual);
}

#[test]
fn toggle_selection_should_update_containers() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let target = templated_list_box();
    target.set_items_source(numbered(10));
    target.set_selection_mode(SelectionMode::TOGGLE);
    target.set_item_template(text_block_template(None, 10.0));

    let _root = prepare(&target);

    let lb_items = logical_list_box_items(&target);

    let item = lb_items[0].clone();

    assert!(!item.is_selected());

    raise_pressed_event(&mouse, &item, MouseButton::Left);

    assert!(item.is_selected());

    raise_pressed_event(&mouse, &item, MouseButton::Left);

    assert!(!item.is_selected());
}

#[test]
fn can_decrease_number_of_materialized_items_by_removing_from_source_collection() {
    let _app = start();
    let items = numbered_list(20);
    let target = templated_list_box();
    target.set_items_source(Some(items.clone().into()));
    target.set_item_template(text_block_template(None, 10.0));

    let _root = prepare(&target);
    set_scroll_offset(&target, Vector::new(0.0, 1.0));

    items.remove_range(0, 11);
}

#[test]
fn layout_manager_should_measure_arrange_all() {
    let _app = start_styled_window();
    let items = Rc::new(FerroList::from_items((1..=7).map(|v| v.to_string())));

    let wnd = Window::new();
    wnd.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);

    wnd.set_is_visible(true);

    let target = templated_list_box();

    wnd.set_content(Some(Control::boxed(target.clone())));

    let lm = wnd.layout_manager();

    target.set_height(110.0);
    target.set_width(50.0);
    target.set_data_context(Some(Rc::new(ItemsSource::from(items.clone())) as BoxedValue));

    target.set_item_template(Some(FuncDataTemplate::new(
        |data| data.is_some(),
        |_, _| {
            let tb = TextBlock::new();
            tb.set_height(10.0);
            tb.set_width(30.0);
            tb.bind_binding(TextBlock::text_property().as_property(), &ReflectionBinding::default());
            Some(tb.upcast())
        },
        true,
    )));

    lm.execute_initial_layout_pass();

    target.set_items_source(Some(items.clone().into()));

    lm.execute_layout_pass();

    items.insert(3, "3+".to_string());
    lm.execute_layout_pass();

    items.insert(4, "4+".to_string());
    lm.execute_layout_pass();

    // RESET
    items.clear();
    for i in 1..=7 {
        items.add(i.to_string());
    }

    // working bit better with this line no outof memory or remaining to arrange/measure ???
    // lm.execute_layout_pass();

    items.insert(2, "2+".to_string());

    lm.execute_layout_pass();
    // after few more layout cycles layoutmanager shouldn't hold any more visual for measure/arrange
    lm.execute_layout_pass();
    lm.execute_layout_pass();

    assert_eq!(0, lm.to_measure_count());
    assert_eq!(0, lm.to_arrange_count());
}

#[test]
fn list_box_should_be_valid_after_remove_of_item_in_non_visible_area() {
    let _app = start_styled_window();
    let items = Rc::new(FerroList::from_items((1..=30).map(|v| v.to_string())));

    let wnd = visible_window();

    let target = templated_list_box();
    target.set_auto_scroll_to_selected_item(true);
    target.set_height(100.0);
    target.set_width(50.0);
    target.set_item_template(border_template());
    target.set_items_source(Some(items.clone().into()));
    wnd.set_content(Some(Control::boxed(target.clone())));

    let lm = wnd.layout_manager();

    lm.execute_initial_layout_pass();

    // Select last / scroll to last item.
    target.set_selected_item(boxed_str(&items.get(items.count() - 1)));

    lm.execute_layout_pass();

    // Remove the first item (in non realized area of the list box).
    items.remove(&"1".to_string());
    lm.execute_layout_pass();

    Dispatcher::ui_thread().run_jobs(None);

    let count = items.count() as i32;
    assert_eq!(Some("30".to_string()), data_context_string(target.container_from_index(count - 1)));
    assert_eq!(Some("29".to_string()), data_context_string(target.container_from_index(count - 2)));
    assert_eq!(Some("28".to_string()), data_context_string(target.container_from_index(count - 3)));
    assert_eq!(Some("27".to_string()), data_context_string(target.container_from_index(count - 4)));
    assert_eq!(Some("26".to_string()), data_context_string(target.container_from_index(count - 5)));
}

#[test]
fn clicking_item_should_raise_bring_into_view_for_correct_control() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    // Issue #3934
    let target = templated_list_box();
    target.set_items_source(numbered(10));
    target.set_item_template(text_block_template(None, 10.0));
    target.set_selection_mode(SelectionMode::ALWAYS_SELECTED);

    let _root = prepare(&target);

    Dispatcher::ui_thread().run_jobs(None);

    // First an item that is not index 0 must be selected.
    mouse.click(&panel_child(&target, 1));

    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(1, target.selection().anchor_index());

    // We're going to be clicking on item 9.
    let item = panel_child(&target, 9);
    let raised = Rc::new(Cell::new(0));

    // Make sure a `RequestBringIntoView` event is raised for item 9. It
    // won't be handled by the scroll content presenter as the item is
    // already visible, so we don't need to see handled events too. Issue
    // #3934 failed here because item 0 was being scrolled into view due to
    // the always selected mode.
    let handler_raised = raised.clone();
    let handler_item: Ref<Visual> = item.clone().upcast();
    target.add_handler(Control::request_bring_into_view_event(), move |_, e| {
        assert_eq!(Some(&handler_item), e.target_object.as_ref());
        handler_raised.set(handler_raised.get() + 1);
    });

    // Click item 9.
    mouse.click(&item);

    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(1, raised.get());
}

#[test]
fn adding_and_selecting_item_with_auto_scroll_to_selected_item_should_not_hide_first_item() {
    let _app = start_styled_window();
    let items = Rc::new(FerroList::<String>::new());

    let wnd = visible_window();

    let target = templated_list_box();
    target.set_vertical_alignment(VerticalAlignment::Top);
    target.set_auto_scroll_to_selected_item(true);
    target.set_width(50.0);
    target.set_item_template(border_template());
    target.set_items_source(Some(items.clone().into()));
    wnd.set_content(Some(Control::boxed(target.clone())));

    let lm = wnd.layout_manager();

    lm.execute_initial_layout_pass();

    let panel = panel_of(&target);

    items.add("Item 1".to_string());
    target.selection().select(0);
    lm.execute_layout_pass();

    assert_eq!(1, panel.children().count());

    items.add("Item 2".to_string());
    target.selection().select(1);
    lm.execute_layout_pass();

    assert_eq!(2, panel.children().count());

    // Make sure we have enough space to show all items.
    let children = panel.children().to_vec();
    assert!(panel.bounds().height >= children.iter().map(|c| c.bounds().height).sum::<f64>());

    // Make sure we show items and they completely visible, not only
    // partially.
    assert!(
        children[0].bounds().top() >= 0.0 && children[0].bounds().bottom() <= panel.bounds().height,
        "first item is not completely visible!"
    );
    assert!(
        children[1].bounds().top() >= 0.0 && children[1].bounds().bottom() <= panel.bounds().height,
        "second item is not completely visible!"
    );
}

#[test]
fn initial_binding_of_selected_items_should_not_cause_write_to_selected_items() {
    let _app = start();
    ItemsSource::register_binding_conversion::<ItemsSource>();
    ValueTypes::register_conversion::<SelectedItemsList, Option<SelectedItemsList>>(|x| Some(Some(x.clone())));
    let target = ListBox::new();
    target.bind_binding(ItemsControl::items_source_property().as_property(), &ReflectionBinding::new("Items"));
    target.bind_binding(ListBox::selected_items_property().as_property(), &ReflectionBinding::new("SelectedItems"));

    let view_model = Model::new_model(SelectedItemsViewModel {
        items: ItemsSource::from_strs(["Foo", "Bar", "Baz "]),
        selected_items: SelectedItemsList::from_items([boxed_str("Bar")]),
    });

    let raised = Rc::new(Cell::new(0));

    let handler_raised = raised.clone();
    view_model.selected_items.add_collection_changed(Rc::new(move |_| handler_raised.set(handler_raised.get() + 1)));

    let data_context: BoxedValue = view_model.clone();
    target.set_data_context(Some(data_context));

    let strings = |items: Vec<Option<BoxedValue>>| -> Vec<Option<String>> {
        items.iter().map(|x| x.as_ref().and_then(string_of)).collect()
    };
    let expected = vec![Some("Bar".to_string())];

    assert_eq!(0, raised.get());
    assert_eq!(expected, strings(view_model.selected_items.to_vec()));
    assert_eq!(expected, strings(target.selected_items().unwrap().to_vec()));
    assert_eq!(expected, strings(target.selection().selected_items().to_vec()));
}

#[test]
fn content_can_be_bound_in_item_container_theme() {
    let _app = start();
    let items = [ItemViewModel::new("Foo"), ItemViewModel::new("Bar")];
    let theme = ControlTheme::with_setters(
        ListBoxItem::TYPE,
        [Setter::new_binding_base(
            ContentControl::content_property().as_property(),
            ReflectionBinding::new("Caption"),
        )],
    );

    let target = templated_list_box();
    target.set_items_source(Some(ItemsSource::from_items(items.iter().map(|item| {
        let item: BoxedValue = item.clone();
        Some(item)
    }))));
    target.set_item_container_theme(Some(theme));

    let _root = prepare(&target);

    let containers: Vec<Ref<ListBoxItem>> =
        target.get_realized_containers().into_iter().map(|x| x.cast::<ListBoxItem>().unwrap()).collect();
    assert_eq!(2, containers.len());
    assert_eq!(Some("Foo".to_string()), containers[0].content().as_ref().and_then(string_of));
    assert_eq!(Some("Bar".to_string()), containers[1].content().as_ref().and_then(string_of));
}

#[test]
fn selected_item_validation() {
    let _app = start();
    let target = templated_list_box();
    target.set_items_source(strs(&["Foo"]));
    target.set_item_template(canvas_template(None));
    target.set_selection_mode(SelectionMode::ALWAYS_SELECTED);

    let _root = prepare(&target);

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
    let errors = DataValidationErrors::get_errors(&target).unwrap();
    assert_eq!(1, errors.len());
    let error = errors[0].clone();
    let value: &dyn AnyValue = &*error;
    assert_eq!(Some(&exception), value.downcast_ref::<BindingError>());
}

#[test]
fn handles_resetting_items() {
    let _app = start();
    let items = ResettingCollection::new(100);
    let target = templated_list_box();
    target.set_items_source(Some(ItemsSource::new(items.clone())));
    target.set_item_template(canvas_template(Some(10.0)));

    let _root = prepare(&target);

    let realized = realized_data_contexts(&target);

    assert_eq!((0..10).map(|x| format!("Item{x}")).collect::<Vec<_>>(), realized);

    items.reverse();
    layout(&target);

    let realized = realized_data_contexts(&target);

    assert_eq!((0..10).map(|x| format!("Item{}", 99 - x)).collect::<Vec<_>>(), realized);
}

#[test]
fn handles_resetting_items_with_existing_selection_and_auto_scroll_to_selected_item() {
    let _app = start();
    let items = ResettingCollection::new(100);
    let target = templated_list_box();
    target.set_items_source(Some(ItemsSource::new(items.clone())));
    target.set_item_template(canvas_template(Some(10.0)));
    target.set_auto_scroll_to_selected_item(true);
    target.set_selected_index(1);

    let _root = prepare(&target);

    let realized = realized_data_contexts(&target);

    assert_eq!((0..10).map(|x| format!("Item{x}")).collect::<Vec<_>>(), realized);

    items.reverse();
    layout(&target);

    Dispatcher::ui_thread().run_jobs(None);

    let realized = realized_data_contexts(&target);

    // "Item1" should remain selected, and now be at the bottom of the
    // viewport.
    assert_eq!((0..10).map(|x| format!("Item{}", 10 - x)).collect::<Vec<_>>(), realized);
}

#[test]
fn arrow_keys_should_move_selection_vertical() {
    let _app = start_with_focus();
    let target = templated_list_box();
    target.set_items_source(numbered(10));
    target.set_item_template(text_block_template(None, 10.0));
    target.set_selected_index(0);

    let _root = prepare(&target);

    raise_key_event(&target, Key::Down, KeyModifiers::NONE);
    assert_eq!(1, target.selected_index());

    raise_key_event(&target, Key::Down, KeyModifiers::NONE);
    assert_eq!(2, target.selected_index());

    raise_key_event(&target, Key::Up, KeyModifiers::NONE);
    assert_eq!(1, target.selected_index());
}

#[test]
fn arrow_keys_should_move_selection_horizontal() {
    let _app = start_with_focus();
    let target = templated_list_box();
    target.set_items_source(numbered(10));
    let items_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| {
        let panel = VirtualizingStackPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        Some(panel.upcast::<Panel>())
    });
    target.set_items_panel(items_panel);
    target.set_item_template(text_block_template(None, 10.0));
    target.set_selected_index(0);

    let _root = prepare(&target);

    raise_key_event(&target, Key::Right, KeyModifiers::NONE);
    assert_eq!(1, target.selected_index());

    raise_key_event(&target, Key::Right, KeyModifiers::NONE);
    assert_eq!(2, target.selected_index());

    raise_key_event(&target, Key::Left, KeyModifiers::NONE);
    assert_eq!(1, target.selected_index());
}

#[test]
fn arrow_keys_should_focus_selection() {
    let _app = start_with_focus();
    let target = templated_list_box();
    target.set_items_source(numbered(10));
    target.set_item_template(text_block_template(None, 10.0));
    target.set_selected_index(0);

    let _root = prepare(&target);

    raise_key_event(&target, Key::Down, KeyModifiers::NONE);
    assert!(is_focused(target.container_from_index(1)));

    raise_key_event(&target, Key::Down, KeyModifiers::NONE);
    assert!(is_focused(target.container_from_index(2)));

    raise_key_event(&target, Key::Up, KeyModifiers::NONE);
    assert!(is_focused(target.container_from_index(1)));
}

#[test]
fn down_key_selecting_from_no_selection_and_no_focus_selects_from_start() {
    let _app = start_with_focus();
    let target = templated_list_box();
    target.set_items_source(strs(&["Foo", "Bar", "Baz"]));
    target.set_width(100.0);
    target.set_height(100.0);

    let _root = prepare(&target);

    raise_key_event(&target, Key::Down, KeyModifiers::NONE);

    assert_eq!(0, target.selected_index());
}

#[test]
fn down_key_selecting_from_no_selection_selects_from_focus() {
    let _app = start_with_focus();
    let target = templated_list_box();
    target.set_items_source(strs(&["Foo", "Bar", "Baz"]));
    target.set_width(100.0);
    target.set_height(100.0);

    let _root = prepare(&target);

    target.container_from_index(1).unwrap().focus();
    raise_key_event(&target, Key::Down, KeyModifiers::NONE);

    assert_eq!(2, target.selected_index());
}

#[test]
fn ctrl_down_key_moves_focus_but_not_selection() {
    let _app = start_with_focus();
    let target = templated_list_box();
    target.set_items_source(strs(&["Foo", "Bar", "Baz"]));
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_selected_index(0);

    let _root = prepare(&target);

    target.container_from_index(0).unwrap().focus();
    raise_key_event(&target, Key::Down, KeyModifiers::CONTROL);

    assert_eq!(0, target.selected_index());
    assert!(is_focused(target.container_from_index(1)));
}

#[test]
fn down_key_brings_unrealized_selection_into_view() {
    let _app = start_with_focus();
    let target = templated_list_box();
    target.set_items_source(numbered(100));
    target.set_item_template(text_block_template(Some(20.0), 10.0));
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_selected_index(0);

    let _root = prepare(&target);

    target.container_from_index(0).unwrap().focus();
    set_scroll_offset(&target, Vector::new(0.0, 100.0));
    layout(&target);

    let panel = target.items_panel_root().unwrap().cast::<VirtualizingStackPanel>().unwrap();
    assert_eq!(10, panel.first_realized_index());

    raise_key_event(&target, Key::Down, KeyModifiers::NONE);

    assert_eq!(1, target.selected_index());
    assert!(is_focused(target.container_from_index(1)));
    assert_eq!(Vector::new(0.0, 10.0), scroll_offset(&target));
}

#[test]
fn wrap_selection_should_wrap() {
    let _app = start_with_focus();
    let mouse = MouseTestHelper::new();
    let target = templated_list_box();
    target.set_items_source(numbered(10));
    target.set_item_template(text_block_template(None, 10.0));
    target.set_wrap_selection(true);

    let _root = prepare(&target);

    let lb_items = logical_list_box_items(&target);

    let first = lb_items.first().unwrap().clone();
    let before_last = lb_items[lb_items.len() - 2].clone();
    let last = lb_items.last().unwrap().clone();

    first.focus();

    raise_pressed_event(&mouse, &first, MouseButton::Left);
    assert!(first.is_selected());

    raise_key_event(&target, Key::Up, KeyModifiers::NONE);
    assert!(last.is_selected());

    raise_key_event(&target, Key::Up, KeyModifiers::NONE);
    assert!(before_last.is_selected());

    raise_key_event(&target, Key::Down, KeyModifiers::NONE);
    assert!(last.is_selected());

    raise_key_event(&target, Key::Down, KeyModifiers::NONE);
    assert!(first.is_selected());

    target.set_wrap_selection(false);
    raise_key_event(&target, Key::Up, KeyModifiers::NONE);

    assert!(first.is_selected());
}

fn container_prepared_is_raised_for_each_container_on_layout(target: Ref<ListBox>) {
    let result: Rc<RefCell<Vec<Ref<Control>>>> = Rc::new(RefCell::new(Vec::new()));
    let index = Rc::new(Cell::new(0));

    let handler_result = result.clone();
    target.container_prepared(move |e| {
        assert_eq!(index.get(), e.index());
        index.set(index.get() + 1);
        handler_result.borrow_mut().push(e.container().clone());
    });

    let _root = prepare(&target);

    assert_eq!(3, result.borrow().len());
    assert_eq!(target.get_realized_containers(), *result.borrow());
}

#[test]
fn container_prepared_is_raised_for_each_item_container_on_layout() {
    let _app = start();

    let target = templated_list_box();
    for item in ["Foo", "Bar", "Baz"] {
        target.items().add(boxed_str(item));
    }

    container_prepared_is_raised_for_each_container_on_layout(target);
}

#[test]
fn container_prepared_is_raised_for_each_items_source_container_on_layout() {
    let _app = start();

    let target = templated_list_box();
    target.set_items_source(strs(&["Foo", "Bar", "Baz"]));

    container_prepared_is_raised_for_each_container_on_layout(target);
}

fn data_list() -> Rc<FerroList<String>> {
    Rc::new(FerroList::from_items(["Foo", "Bar", "Baz"].map(String::from)))
}

#[test]
fn container_prepared_is_raised_for_added_item() {
    let _app = start();

    let data = data_list();
    let target = templated_list_box();
    target.set_items_source(Some(data.clone().into()));

    let _root = prepare(&target);

    let result: Rc<RefCell<Vec<Ref<Control>>>> = Rc::new(RefCell::new(Vec::new()));

    let handler_result = result.clone();
    target.container_prepared(move |e| {
        assert_eq!(3, e.index());
        handler_result.borrow_mut().push(e.container().clone());
    });

    data.add("Qux".to_string());
    layout(&target);

    assert_eq!(1, result.borrow().len());
}

#[test]
fn container_index_changed_is_raised_when_item_added() {
    let _app = start();

    let data = data_list();
    let target = templated_list_box();
    target.set_items_source(Some(data.clone().into()));

    let _root = prepare(&target);

    let result: Rc<RefCell<Vec<Ref<Control>>>> = Rc::new(RefCell::new(Vec::new()));
    let index = Rc::new(Cell::new(1));

    let handler_result = result.clone();
    target.container_index_changed(move |e| {
        assert_eq!(index.get(), e.old_index());
        index.set(index.get() + 1);
        assert_eq!(index.get(), e.new_index());
        handler_result.borrow_mut().push(e.container().clone());
    });

    data.insert(1, "Qux".to_string());
    layout(&target);

    assert_eq!(2, result.borrow().len());
    assert_eq!(target.get_realized_containers().into_iter().skip(2).collect::<Vec<_>>(), *result.borrow());
}

#[test]
fn container_clearing_is_raised_when_item_removed() {
    let _app = start();

    let data = data_list();
    let target = templated_list_box();
    target.set_items_source(Some(data.clone().into()));

    let _root = prepare(&target);

    let expected = target.container_from_index(1);
    let raised = Rc::new(Cell::new(0));

    let handler_raised = raised.clone();
    target.container_clearing(move |e| {
        assert_eq!(expected.as_ref(), Some(e.container()));
        handler_raised.set(handler_raised.get() + 1);
    });

    data.remove_at(1);
    layout(&target);

    assert_eq!(1, raised.get());
}

/// The list box and the button of the tab navigation tests, docked in a
/// panel.
fn tab_navigation_target() -> (Ref<ListBox>, Ref<Button>, Ref<DockPanel>) {
    let target = templated_list_box();
    for item in ["Foo", "Bar", "Baz"] {
        target.items().add(boxed_str(item));
    }

    let button = Button::new();
    button.set_content(boxed_str("Button"));
    DockPanel::set_dock(&button, Dock::Top);

    let panel = DockPanel::new();
    panel.children().add(button.clone());
    panel.children().add(target.clone());

    (target, button, panel)
}

#[test]
fn tab_navigation_should_move_to_first_item_when_no_anchor_element_selected() {
    let _app = start_with_focus();

    let (target, button, panel) = tab_navigation_target();

    let root = test_root();
    root.set_child(panel);

    let navigation = KeyboardNavigationHandler::new();
    navigation.set_owner(&root.clone().upcast());

    root.execute_initial_layout_pass();

    button.focus();
    raise_key_event(&button, Key::Tab, KeyModifiers::NONE);

    let item = target.container_from_index(0);
    assert_eq!(item.map(|item| item.upcast::<InputElement>()), focused_element(&root));
}

#[test]
fn tab_navigation_should_move_to_anchor_element() {
    let _app = start_with_focus();

    let (target, button, panel) = tab_navigation_target();

    let root = test_root();
    root.set_width(1000.0);
    root.set_height(1000.0);
    root.set_child(panel);

    let navigation = KeyboardNavigationHandler::new();
    navigation.set_owner(&root.clone().upcast());

    root.execute_initial_layout_pass();

    button.focus();
    target.selection().set_anchor_index(1);
    raise_key_event(&button, Key::Tab, KeyModifiers::NONE);

    let item = target.container_from_index(1);
    assert!(item.is_some());
    let item = item.unwrap();
    assert_eq!(Some(item.clone().upcast::<InputElement>()), focused_element(&root));

    raise_key_event(&item, Key::Tab, KeyModifiers::NONE);

    assert_eq!(Some(button.clone().upcast::<InputElement>()), focused_element(&root));

    target.selection().set_anchor_index(2);
    raise_key_event(&button, Key::Tab, KeyModifiers::NONE);

    let item = target.container_from_index(2);
    assert_eq!(item.map(|item| item.upcast::<InputElement>()), focused_element(&root));
}

#[test]
fn reads_only_realized_items_from_items_source() {
    let _app = start();

    let data = DataVirtualizingList::new();
    let target = templated_list_box();
    target.set_items_source(Some(ItemsSource::new(data.clone())));

    let _root = prepare(&target);

    let panel = target.items_panel_root().unwrap();
    assert_eq!(panel.get_type(), VirtualizingStackPanel::TYPE);
    let panel = panel.cast::<VirtualizingStackPanel>().unwrap();
    // The reference expects 7 realized items with the line height of its
    // test font; the number of items that fit depends on the font of the
    // text test scope, so it is derived from the height of a container.
    let item_height = target.container_from_index(0).unwrap().bounds().height;
    let expected_count = (100.0 / item_height).ceil() as i32;
    assert!(expected_count < 100);
    assert_eq!(0, panel.first_realized_index());
    assert_eq!(expected_count - 1, panel.last_realized_index());

    assert_eq!((0..expected_count).map(|x| format!("Item{x}")).collect::<Vec<_>>(), data.get_realized_items());
}

#[test]
fn list_box_item_should_not_block_tapped_events() {
    // #13474
    let _app = start();
    let mouse = MouseTestHelper::new();

    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
    let next_stamp = Cell::new(1u64);
    let stamp = || {
        let stamp = next_stamp.get();
        next_stamp.set(stamp + 1);
        stamp
    };

    let target = templated_list_box();
    target.set_items_source(numbered(10));
    target.set_selection_mode(SelectionMode::TOGGLE);
    target.set_item_template(text_block_template(None, 10.0));

    let _root = prepare(&target);

    let lb_items = logical_list_box_items(&target);

    let item = lb_items[0].clone();

    let tapped_count = Rc::new(Cell::new(0));
    let handler_tapped_count = tapped_count.clone();
    target.tapped(move |_, _| handler_tapped_count.set(handler_tapped_count.get() + 1));

    mouse.click(&item);
    assert_eq!(1, tapped_count.get());

    // Raise pointer pressed and pointer released events with the left button
    // pressed. The touch test helper assumes no button pressed, which
    // prevents it from generating tapped events.
    let item_visual: Ref<Visual> = item.clone().upcast();
    let touch_pointer: Rc<dyn IPointer> = pointer.clone();

    item.raise_event(&PointerPressedEventArgs::new(
        item.clone().upcast::<Interactive>(),
        touch_pointer.clone(),
        &item_visual,
        Point::default(),
        stamp(),
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    ));

    item.raise_event(&PointerReleasedEventArgs::new(
        item.clone().upcast::<Interactive>(),
        touch_pointer,
        &item_visual,
        Point::default(),
        stamp(),
        PointerPointProperties::NONE,
        KeyModifiers::NONE,
        MouseButton::Left,
    ));

    assert_eq!(2, tapped_count.get());
}
