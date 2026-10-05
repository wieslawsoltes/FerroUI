//! Tests of the list box in single selection mode.

use crate::mouse_test_helper::MouseTestHelper;
use crate::presenters::{ItemsPresenter, ScrollContentPresenter};
use crate::primitives::SelectingItemsControl;
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot, TestScope};
use crate::{Border, ContentControl, Control, ItemsSource, ListBox, ListBoxItem, ScrollViewer, SelectionMode, TextBlock};
use ferroui_base::data::core::Untyped;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingMode, IndexerBinding, ReflectionBinding};
use ferroui_base::input::gesture_recognizers::ScrollGestureRecognizer;
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, NavigationMethod, PointerType,
};
use ferroui_base::platform::DefaultPlatformSettings;
use ferroui_base::{ferro_model, BoxedValue, FerroLocator, Point, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

struct TestStackOverflowViewModel {
    items: Vec<String>,
    setter_invoked_count: Cell<i32>,
    selected_item: RefCell<Option<String>>,
    property_changed: Event<str>,
}

impl TestStackOverflowViewModel {
    const MAX_INVOKED_COUNT: i32 = 1000;

    fn new(items: &[&str]) -> Rc<Self> {
        Model::new_model(Self {
            items: items.iter().map(|x| x.to_string()).collect(),
            setter_invoked_count: Cell::new(0),
            selected_item: RefCell::new(None),
            property_changed: Event::new(),
        })
    }

    fn selected_item(&self) -> Option<BoxedValue> {
        self.selected_item.borrow().as_deref().and_then(boxed_str)
    }

    fn set_selected_item(&self, value: Option<BoxedValue>) {
        let value = value.as_ref().and_then(string_of);

        if *self.selected_item.borrow() != value {
            self.setter_invoked_count.set(self.setter_invoked_count.get() + 1);

            let index = self.items.iter().position(|x| Some(x) == value.as_ref());

            let selected_item = match index {
                Some(index) if Self::MAX_INVOKED_COUNT > self.setter_invoked_count.get() && index > 0 => {
                    Some(self.items[index - 1].clone())
                }
                _ => value,
            };
            *self.selected_item.borrow_mut() = selected_item;

            self.property_changed.raise("SelectedItem");
        }
    }
}

impl INotifyPropertyChanged for TestStackOverflowViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(TestStackOverflowViewModel, |b| b
    .notify_property_changed()
    .property::<Untyped>("SelectedItem", |vm| vm.selected_item(), |vm, v| vm.set_selected_item(v)));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn start() -> TestScope {
    test_scope()
}

/// Registers the hotkey configuration of the platform, as the tests of the
/// reference that need one do.
fn bind_hotkey_configuration() {
    FerroLocator::current_mutable()
        .bind::<PlatformHotkeyConfiguration>()
        .to_constant(Rc::new(PlatformHotkeyConfiguration::default()));
}

fn create_list_box_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ListBox>(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_template(Some(create_scroll_viewer_template()));
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));
        scroll_viewer.upcast()
    })
}

fn create_scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|parent, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let property = ContentControl::content_property().as_property();
        presenter.bind_binding(
            crate::presenters::ContentPresenter::content_property().as_property(),
            &IndexerBinding::new(parent.clone().upcast(), property, BindingMode::OneWay),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn create_target() -> Ref<ListBox> {
    let target = ListBox::new();
    target.set_template(Some(create_list_box_template()));
    target.set_items_source(Some(ItemsSource::from_strs(["Foo", "Bar", "Baz "])));
    target
}

/// A data template for strings that creates a text block.
fn text_block_template() -> Option<Rc<dyn IDataTemplate>> {
    Some(FuncDataTemplate::for_type::<String>(
        |_, _| {
            let text_block = TextBlock::new();
            text_block.set_width(20.0);
            text_block.set_height(10.0);
            Some(text_block.upcast())
        },
        false,
    ))
}

fn prepare(target: &Ref<ListBox>) -> Ref<TestRoot> {
    target.set_height(100.0);
    target.set_width(100.0);
    let root = TestRoot::new();
    // The test application of the reference supplies the platform settings.
    root.set_platform_settings(Some(Rc::new(DefaultPlatformSettings::new())));
    root.set_child(target.clone());
    root.execute_initial_layout_pass();
    root
}

fn panel_child(target: &ListBox, index: usize) -> Ref<Control> {
    target.presenter().expect("no presenter").panel().expect("no panel").children().get(index)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn focusing_item_with_tab_should_not_select_it() {
    let _app = start();
    let target = create_target();
    // The platform settings of the root read the hotkey configuration, which
    // the test application of the reference always has.
    bind_hotkey_configuration();

    let _root = prepare(&target);

    let mut e = FocusChangedEventArgs::new(InputElement::got_focus_event());
    e.navigation_method = NavigationMethod::Tab;
    panel_child(&target, 0).raise_event(&e);

    assert_eq!(-1, target.selected_index());
}

#[test]
fn pressing_space_on_focused_item_with_ctrl_pressed_should_select_it() {
    let _app = start();
    let target = create_target();
    bind_hotkey_configuration();
    let _root = prepare(&target);

    let mut e = FocusChangedEventArgs::new(InputElement::got_focus_event());
    e.navigation_method = NavigationMethod::Directional;
    e.key_modifiers = KeyModifiers::CONTROL;
    panel_child(&target, 0).raise_event(&e);

    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key = Key::Space;
    e.key_modifiers = KeyModifiers::CONTROL;
    panel_child(&target, 0).raise_event(&e);

    assert_eq!(0, target.selected_index());
}

#[test]
fn clicking_item_should_select_it() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let target = create_target();
    bind_hotkey_configuration();
    let _root = prepare(&target);
    mouse.click(&panel_child(&target, 0));

    assert_eq!(0, target.selected_index());
}

#[test]
fn pen_right_press_item_should_select_it() {
    let _app = start();
    let pen = MouseTestHelper::with_pointer_type(PointerType::Pen);
    let target = create_target();
    target.set_item_template(text_block_template());
    bind_hotkey_configuration();
    let _root = prepare(&target);
    let item = panel_child(&target, 0);
    pen.down_with(&item, &item, MouseButton::Right, None, KeyModifiers::NONE, 1);

    assert_eq!(0, target.selected_index());
}

#[test]
fn pen_left_press_item_should_not_select_it() {
    let _app = start();
    let pen = MouseTestHelper::with_pointer_type(PointerType::Pen);
    let target = create_target();
    target.set_item_template(text_block_template());
    bind_hotkey_configuration();
    let _root = prepare(&target);
    pen.down(&panel_child(&target, 0));

    assert_eq!(-1, target.selected_index());
}

fn pointer_right_click_should_select_item_and_open_context(type_: PointerType) {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let pen = MouseTestHelper::with_pointer_type(PointerType::Pen);
    let target = create_target();
    target.set_item_template(Some(FuncDataTemplate::for_type::<String>(
        |_, _| {
            let border = Border::new();
            border.set_height(10.0);
            Some(border.upcast())
        },
        false,
    )));
    let recognizer = ScrollGestureRecognizer::new();
    recognizer.set_can_vertically_scroll(true);
    recognizer.set_scroll_start_distance(50);
    target.gesture_recognizers().add(recognizer);
    bind_hotkey_configuration();
    let _root = prepare(&target);

    let context_raised = Rc::new(Cell::new(false));
    let handler_context_raised = context_raised.clone();
    target.add_handler(InputElement::context_requested_event(), move |_, args| {
        handler_context_raised.set(true);
        args.set_handled(true);
    });

    let pointer = if type_ == PointerType::Mouse { &mouse } else { &pen };
    let item = panel_child(&target, 0);
    pointer.click_with(&item, &item, MouseButton::Right, Some(Point::new(5.0, 5.0)), KeyModifiers::NONE);

    assert!(context_raised.get());
    assert_eq!(0, target.selected_index());
}

#[test]
fn pointer_right_click_should_select_item_and_open_context_1() {
    pointer_right_click_should_select_item_and_open_context(PointerType::Mouse);
}

#[test]
fn pointer_right_click_should_select_item_and_open_context_2() {
    pointer_right_click_should_select_item_and_open_context(PointerType::Pen);
}

#[test]
fn clicking_selected_item_should_not_deselect_it() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let target = create_target();
    bind_hotkey_configuration();
    let _root = prepare(&target);
    target.set_selected_index(0);

    mouse.click(&panel_child(&target, 0));

    assert_eq!(0, target.selected_index());
}

#[test]
fn clicking_item_should_select_it_when_selection_mode_toggle() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let target = create_target();
    target.set_selection_mode(SelectionMode::SINGLE | SelectionMode::TOGGLE);
    bind_hotkey_configuration();
    let _root = prepare(&target);

    mouse.click(&panel_child(&target, 0));

    assert_eq!(0, target.selected_index());
}

#[test]
fn clicking_selected_item_should_deselect_it_when_selection_mode_toggle() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let target = create_target();
    target.set_selection_mode(SelectionMode::TOGGLE);

    bind_hotkey_configuration();
    let _root = prepare(&target);
    target.set_selected_index(0);

    mouse.click(&panel_child(&target, 0));

    assert_eq!(-1, target.selected_index());
}

#[test]
fn clicking_selected_item_should_not_deselect_it_when_selection_mode_toggle_always_selected() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let target = create_target();
    target.set_selection_mode(SelectionMode::TOGGLE | SelectionMode::ALWAYS_SELECTED);
    bind_hotkey_configuration();
    let _root = prepare(&target);
    target.set_selected_index(0);

    mouse.click(&panel_child(&target, 0));

    assert_eq!(0, target.selected_index());
}

#[test]
fn clicking_another_item_should_select_it_when_selection_mode_toggle() {
    let _app = start();
    let mouse = MouseTestHelper::new();
    let target = create_target();
    target.set_selection_mode(SelectionMode::SINGLE | SelectionMode::TOGGLE);
    bind_hotkey_configuration();
    let _root = prepare(&target);
    target.set_selected_index(1);

    mouse.click(&panel_child(&target, 0));

    assert_eq!(0, target.selected_index());
}

#[test]
fn setting_item_is_selected_sets_list_box_selection() {
    let _app = start();
    let target = create_target();
    bind_hotkey_configuration();

    let _root = prepare(&target);

    target.logical_children().get(1).cast::<ListBoxItem>().unwrap().set_is_selected(true);

    assert_eq!(Some("Bar".to_string()), target.selected_item().as_ref().and_then(string_of));
    assert_eq!(1, target.selected_index());
}

#[test]
fn selected_item_should_not_cause_stack_overflow() {
    let _app = start();
    let view_model = TestStackOverflowViewModel::new(&["foo", "bar", "baz"]);

    let target = ListBox::new();
    target.set_template(Some(create_list_box_template()));
    let data_context: BoxedValue = view_model.clone();
    target.set_data_context(Some(data_context));
    target.set_items_source(Some(ItemsSource::from_strs(view_model.items.iter().map(|x| x.as_str()))));

    target.bind_binding(
        SelectingItemsControl::selected_item_property().as_property(),
        &ReflectionBinding::new("SelectedItem").with_mode(BindingMode::TwoWay),
    );

    assert_eq!(0, view_model.setter_invoked_count.get());

    // In Issue #855, a stack overflow occurred here.
    target.set_selected_item(boxed_str(&view_model.items[2]));

    assert_eq!(Some(view_model.items[1].clone()), target.selected_item().as_ref().and_then(string_of));
    assert_eq!(1, view_model.setter_invoked_count.get());
}
