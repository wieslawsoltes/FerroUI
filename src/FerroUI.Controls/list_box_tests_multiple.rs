//! Tests of the list box with multiple selection.
//!
//! Plain arrays of the reference tests are non-notifying items sources.

use crate::mouse_test_helper::MouseTestHelper;
use crate::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter};
use crate::primitives::SelectingItemsControl;
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::test_support::{string_of, test_scope, TestRoot, TestScope};
use crate::{
    ContentControl, Control, ItemsSource, ListBox, Panel, ScrollViewer, SelectionChangedEventArgs, SelectionMode,
    TextBlock,
};
use ferroui_base::data::{BindingMode, IndexerBinding, ReflectionBinding, RelativeSource, RelativeSourceMode};
use ferroui_base::input::platform::PlatformHotkeyConfiguration;
use ferroui_base::input::{
    IKeyboardDevice, InputElement, Key, KeyEventArgs, KeyModifiers, KeyboardDevice, MouseButton,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::platform::DefaultPlatformSettings;
use ferroui_base::{BoxedValue, FerroLocator, Ref};
use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The added and the removed items of a selection change.
type ReceivedArgs = Rc<RefCell<Option<(Vec<Option<String>>, Vec<Option<String>>)>>>;

fn start() -> TestScope {
    let scope = test_scope();
    FerroLocator::current_mutable()
        .bind::<PlatformHotkeyConfiguration>()
        .to_constant(Rc::new(PlatformHotkeyConfiguration::default()));
    scope
}

/// A test scope with a keyboard device, for the tests that move the focus.
fn start_with_focus() -> TestScope {
    let scope = start();
    FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(KeyboardDevice::new());
    scope
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
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &IndexerBinding::new(
                parent.clone().upcast(),
                ContentControl::content_property().as_property(),
                BindingMode::OneWay,
            ),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

/// A list box with the control template, multiple selection and a size of
/// 100 x 100.
fn create_target(items: &[&str]) -> Ref<ListBox> {
    let target = ListBox::new();
    target.set_template(Some(create_list_box_template()));
    target.set_items_source(Some(ItemsSource::from_strs(items.iter().copied())));
    target.set_selection_mode(SelectionMode::MULTIPLE);
    target.set_width(100.0);
    target.set_height(100.0);
    target
}

/// A data template for strings that creates a text block of 20 x 10.
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

/// Hosts the list box in a test root with the platform settings that the
/// test application of the reference supplies, and lays it out.
fn prepare(target: &Ref<ListBox>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_platform_settings(Some(Rc::new(DefaultPlatformSettings::new())));
    root.set_child(target.clone());
    root.execute_initial_layout_pass();
    root
}

fn panel_of(target: &ListBox) -> Ref<Panel> {
    target.presenter().expect("no presenter").panel().expect("no panel")
}

fn child(target: &ListBox, index: usize) -> Ref<Control> {
    panel_of(target).children().get(index)
}

fn click(helper: &MouseTestHelper, target: &Ref<Control>, button: MouseButton, modifiers: KeyModifiers) {
    helper.click_with(target, target, button, None, modifiers);
}

fn raise_key_event(target: &Interactive, key: Key, input_modifiers: KeyModifiers) {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key_modifiers = input_modifiers;
    e.key = key;
    target.raise_event(&e);
}

fn selected_containers(target: &SelectingItemsControl) -> Vec<i32> {
    target
        .presenter()
        .expect("no presenter")
        .panel()
        .expect("no panel")
        .children()
        .to_vec()
        .iter()
        .map(|x| if x.classes().contains(":selected") { target.index_from_container(x) } else { -1 })
        .filter(|x| *x != -1)
        .collect()
}

fn strings(items: &[Option<BoxedValue>]) -> Vec<Option<String>> {
    items.iter().map(|x| x.as_ref().and_then(string_of)).collect()
}

fn expected(values: &[&str]) -> Vec<Option<String>> {
    values.iter().map(|x| Some(x.to_string())).collect()
}

/// Asserts that the selected items of the list box are the given strings.
#[track_caller]
fn assert_selected_items(values: &[&str], target: &ListBox) {
    let selected_items = target.selected_items().expect("no selected items");
    assert_eq!(expected(values), strings(&selected_items.to_vec()));
}

fn selected_items_count(target: &ListBox) -> usize {
    target.selected_items().expect("no selected items").to_vec().len()
}

fn is_focused(container: Option<Ref<Control>>) -> bool {
    container.expect("no container").is_focused()
}

fn record_selection_changed(target: &ListBox) -> ReceivedArgs {
    let received_args: ReceivedArgs = Rc::new(RefCell::new(None));
    let sink = received_args.clone();
    target.selection_changed(move |_, args: &SelectionChangedEventArgs| {
        *sink.borrow_mut() = Some((strings(args.added_items()), strings(args.removed_items())));
    });
    received_args
}

#[track_caller]
fn verify_added(received_args: &ReceivedArgs, selection: &[&str]) {
    let received_args = received_args.borrow();
    let (added_items, removed_items) = received_args.as_ref().expect("no selection change was received");
    assert_eq!(&expected(selection), added_items);
    assert!(removed_items.is_empty());
}

#[track_caller]
fn verify_removed(received_args: &ReceivedArgs, selection: &str) {
    let received_args = received_args.borrow();
    let (added_items, removed_items) = received_args.as_ref().expect("no selection change was received");
    assert_eq!(&expected(&[selection]), removed_items);
    assert!(added_items.is_empty());
}

/// A two-way binding to the tag of the element itself.
fn tag_binding() -> Rc<ReflectionBinding> {
    let binding = ReflectionBinding::new("Tag").with_mode(BindingMode::TwoWay);
    binding.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::SelfMode)));
    binding
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn shift_selecting_from_no_selection_selects_from_start() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz"]);

    let _root = prepare(&target);

    click(&helper, &child(&target, 2), MouseButton::Left, KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar", "Baz"], &target);
    assert_eq!(vec![0, 1, 2], selected_containers(&target));
}

#[test]
fn ctrl_selecting_raises_selection_changed_events() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz", "Qux"]);

    let _root = prepare(&target);

    let received_args = record_selection_changed(&target);

    click(&helper, &child(&target, 1), MouseButton::Left, KeyModifiers::NONE);

    verify_added(&received_args, &["Bar"]);

    *received_args.borrow_mut() = None;
    click(&helper, &child(&target, 2), MouseButton::Left, KeyModifiers::CONTROL);

    verify_added(&received_args, &["Baz"]);

    *received_args.borrow_mut() = None;
    click(&helper, &child(&target, 3), MouseButton::Left, KeyModifiers::CONTROL);

    verify_added(&received_args, &["Qux"]);

    *received_args.borrow_mut() = None;
    click(&helper, &child(&target, 1), MouseButton::Left, KeyModifiers::CONTROL);

    verify_removed(&received_args, "Bar");
}

#[test]
fn ctrl_selecting_selected_item_with_multiple_selection_active_sets_selected_item_to_next_selection() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz", "Qux"]);

    let _root = prepare(&target);

    click(&helper, &child(&target, 1), MouseButton::Left, KeyModifiers::NONE);
    click(&helper, &child(&target, 2), MouseButton::Left, KeyModifiers::CONTROL);
    click(&helper, &child(&target, 3), MouseButton::Left, KeyModifiers::CONTROL);

    assert_eq!(1, target.selected_index());
    assert_eq!(Some("Bar".to_string()), target.selected_item().as_ref().and_then(string_of));
    assert_selected_items(&["Bar", "Baz", "Qux"], &target);

    click(&helper, &child(&target, 1), MouseButton::Left, KeyModifiers::CONTROL);

    assert_eq!(2, target.selected_index());
    assert_eq!(Some("Baz".to_string()), target.selected_item().as_ref().and_then(string_of));
    assert_selected_items(&["Baz", "Qux"], &target);
}

#[test]
fn ctrl_selecting_non_selected_item_with_multiple_selection_active_leaves_selected_item_the_same() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz"]);

    let _root = prepare(&target);

    click(&helper, &child(&target, 1), MouseButton::Left, KeyModifiers::NONE);
    click(&helper, &child(&target, 2), MouseButton::Left, KeyModifiers::CONTROL);

    assert_eq!(1, target.selected_index());
    assert_eq!(Some("Bar".to_string()), target.selected_item().as_ref().and_then(string_of));

    click(&helper, &child(&target, 2), MouseButton::Left, KeyModifiers::CONTROL);

    assert_eq!(1, target.selected_index());
    assert_eq!(Some("Bar".to_string()), target.selected_item().as_ref().and_then(string_of));
}

#[test]
fn toggle_modifier_and_range_should_select_second_range() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz", "Gap", "Boo", "Far", "Faz"]);

    let _root = prepare(&target);

    // Select first range
    click(&helper, &child(&target, 0), MouseButton::Left, KeyModifiers::NONE);
    click(&helper, &child(&target, 2), MouseButton::Left, KeyModifiers::SHIFT);

    // Select second range
    click(&helper, &child(&target, 4), MouseButton::Left, KeyModifiers::CONTROL);
    click(&helper, &child(&target, 6), MouseButton::Left, KeyModifiers::CONTROL | KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar", "Baz", "Boo", "Far", "Faz"], &target);
}

#[test]
fn should_ctrl_select_correct_item_when_duplicate_items_are_present() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz", "Foo", "Bar", "Baz"]);

    let _root = prepare(&target);

    click(&helper, &child(&target, 3), MouseButton::Left, KeyModifiers::NONE);
    click(&helper, &child(&target, 4), MouseButton::Left, KeyModifiers::CONTROL);

    assert_selected_items(&["Foo", "Bar"], &target);
    assert_eq!(vec![3, 4], selected_containers(&target));
}

#[test]
fn should_shift_select_correct_item_when_duplicates_are_present() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz", "Foo", "Bar", "Baz"]);

    let _root = prepare(&target);

    click(&helper, &child(&target, 3), MouseButton::Left, KeyModifiers::NONE);
    click(&helper, &child(&target, 5), MouseButton::Left, KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar", "Baz"], &target);
    assert_eq!(vec![3, 4, 5], selected_containers(&target));
}

#[test]
fn can_shift_select_all_items_when_duplicates_are_present() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz", "Foo", "Bar", "Baz"]);

    let _root = prepare(&target);

    click(&helper, &child(&target, 0), MouseButton::Left, KeyModifiers::NONE);
    click(&helper, &child(&target, 5), MouseButton::Left, KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar", "Baz", "Foo", "Bar", "Baz"], &target);
    assert_eq!(vec![0, 1, 2, 3, 4, 5], selected_containers(&target));
}

#[test]
fn shift_selecting_raises_selection_changed_events() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz", "Qux"]);

    let _root = prepare(&target);

    let received_args = record_selection_changed(&target);

    click(&helper, &child(&target, 1), MouseButton::Left, KeyModifiers::NONE);

    verify_added(&received_args, &["Bar"]);

    *received_args.borrow_mut() = None;
    click(&helper, &child(&target, 3), MouseButton::Left, KeyModifiers::SHIFT);

    verify_added(&received_args, &["Baz", "Qux"]);

    *received_args.borrow_mut() = None;
    click(&helper, &child(&target, 2), MouseButton::Left, KeyModifiers::SHIFT);

    verify_removed(&received_args, "Qux");
}

#[test]
fn duplicate_items_are_added_to_selected_items_in_order() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz", "Foo", "Bar", "Baz"]);

    let _root = prepare(&target);

    click(&helper, &child(&target, 0), MouseButton::Left, KeyModifiers::NONE);

    assert_selected_items(&["Foo"], &target);

    click(&helper, &child(&target, 4), MouseButton::Left, KeyModifiers::CONTROL);

    assert_selected_items(&["Foo", "Bar"], &target);

    click(&helper, &child(&target, 3), MouseButton::Left, KeyModifiers::CONTROL);

    assert_selected_items(&["Foo", "Bar", "Foo"], &target);

    click(&helper, &child(&target, 1), MouseButton::Left, KeyModifiers::CONTROL);

    assert_selected_items(&["Foo", "Bar", "Foo", "Bar"], &target);
}

#[test]
fn left_click_on_selected_item_should_clear_existing_selection() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz"]);
    target.set_item_template(text_block_template());

    let _root = prepare(&target);

    target.select_all();

    assert_eq!(3, selected_items_count(&target));

    click(&helper, &child(&target, 0), MouseButton::Left, KeyModifiers::NONE);

    assert_eq!(1, selected_items_count(&target));
    assert_selected_items(&["Foo"], &target);
    assert_eq!(vec![0], selected_containers(&target));
}

#[test]
fn right_click_on_selected_item_should_not_clear_existing_selection() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz"]);
    target.set_item_template(text_block_template());

    let _root = prepare(&target);

    target.select_all();

    assert_eq!(3, selected_items_count(&target));

    click(&helper, &child(&target, 0), MouseButton::Right, KeyModifiers::NONE);

    assert_eq!(3, selected_items_count(&target));
}

#[test]
fn right_click_on_unselected_item_should_clear_existing_selection() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz"]);
    target.set_item_template(text_block_template());

    let _root = prepare(&target);

    click(&helper, &child(&target, 0), MouseButton::Left, KeyModifiers::NONE);
    click(&helper, &child(&target, 1), MouseButton::Left, KeyModifiers::SHIFT);

    assert!(target.selected_items().is_some());
    assert_eq!(2, selected_items_count(&target));

    click(&helper, &child(&target, 2), MouseButton::Right, KeyModifiers::NONE);

    assert!(target.selected_items().is_some());
    assert_eq!(1, selected_items_count(&target));
}

#[test]
fn shift_right_click_should_not_select_multiple() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz"]);
    target.set_item_template(text_block_template());

    let _root = prepare(&target);

    click(&helper, &child(&target, 0), MouseButton::Left, KeyModifiers::NONE);
    click(&helper, &child(&target, 2), MouseButton::Right, KeyModifiers::SHIFT);

    assert!(target.selected_items().is_some());
    assert_eq!(1, selected_items_count(&target));
}

#[test]
fn ctrl_right_click_should_not_select_multiple() {
    let _app = start();
    let helper = MouseTestHelper::new();
    let target = create_target(&["Foo", "Bar", "Baz"]);
    target.set_item_template(text_block_template());

    let _root = prepare(&target);

    click(&helper, &child(&target, 0), MouseButton::Left, KeyModifiers::NONE);
    click(&helper, &child(&target, 2), MouseButton::Right, KeyModifiers::CONTROL);

    assert!(target.selected_items().is_some());
    assert_eq!(1, selected_items_count(&target));
}

#[test]
fn shift_arrow_key_selects_range() {
    let _app = start_with_focus();
    let target = create_target(&["Foo", "Bar", "Baz"]);
    target.set_selected_index(0);

    let _root = prepare(&target);

    raise_key_event(&target, Key::Down, KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar"], &target);
    assert_eq!(vec![0, 1], selected_containers(&target));
    assert!(is_focused(target.container_from_index(1)));

    raise_key_event(&target, Key::Down, KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar", "Baz"], &target);
    assert_eq!(vec![0, 1, 2], selected_containers(&target));
    assert!(is_focused(target.container_from_index(2)));

    raise_key_event(&target, Key::Up, KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar"], &target);
    assert_eq!(vec![0, 1], selected_containers(&target));
    assert!(is_focused(target.container_from_index(1)));
}

#[test]
fn shift_down_key_selecting_selects_range_end_from_focus() {
    let _app = start_with_focus();
    let target = create_target(&["Foo", "Bar", "Baz"]);
    target.set_selected_index(0);

    let _root = prepare(&target);

    target.container_from_index(1).unwrap().focus();
    raise_key_event(&target, Key::Down, KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar", "Baz"], &target);
    assert_eq!(vec![0, 1, 2], selected_containers(&target));
    assert!(is_focused(target.container_from_index(2)));
}

#[test]
fn shift_down_key_selecting_selects_range_end_from_focus_moved_with_ctrl_key() {
    let _app = start_with_focus();
    let target = create_target(&["Foo", "Bar", "Baz", "Qux"]);
    target.set_selected_index(0);

    let _root = prepare(&target);

    raise_key_event(&target, Key::Down, KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar"], &target);
    assert_eq!(vec![0, 1], selected_containers(&target));
    assert!(is_focused(target.container_from_index(1)));

    raise_key_event(&target, Key::Down, KeyModifiers::CONTROL);

    assert_selected_items(&["Foo", "Bar"], &target);
    assert_eq!(vec![0, 1], selected_containers(&target));
    assert!(is_focused(target.container_from_index(2)));

    raise_key_event(&target, Key::Down, KeyModifiers::SHIFT);

    assert_selected_items(&["Foo", "Bar", "Baz", "Qux"], &target);
    assert_eq!(vec![0, 1, 2, 3], selected_containers(&target));
    assert!(is_focused(target.container_from_index(3)));
}

#[test]
fn select_all_works_from_no_selection_when_selected_item_is_bound_two_way() {
    // Issue #13676
    let _app = start_with_focus();
    let target = create_target(&["Foo", "Bar", "Baz", "Qux"]);

    let _root = prepare(&target);

    target.bind_binding(SelectingItemsControl::selected_item_property().as_property(), &tag_binding());

    target.select_all();

    assert_eq!(vec![0, 1, 2, 3], target.selection().selected_indexes().to_vec());
    assert_selected_items(&["Foo", "Bar", "Baz", "Qux"], &target);
}

#[test]
fn select_all_works_from_no_selection_when_selected_index_is_bound_two_way() {
    // Issue #13676
    let _app = start_with_focus();
    let target = create_target(&["Foo", "Bar", "Baz", "Qux"]);

    let _root = prepare(&target);

    target.bind_binding(SelectingItemsControl::selected_index_property().as_property(), &tag_binding());

    target.select_all();

    assert_eq!(vec![0, 1, 2, 3], target.selection().selected_indexes().to_vec());
    assert_selected_items(&["Foo", "Bar", "Baz", "Qux"], &target);
}
