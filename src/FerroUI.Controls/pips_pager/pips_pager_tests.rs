//! The pips pager tests of the reference test suite.
//!
//! The reference tests that start the test application with the styled
//! window services do the same here; the simple theme of
//! `simple_theme_should_forward_custom_button_themes` is the test theme
//! with the pips pager theme of `testing::test_theme_pips_pager`, which
//! mirrors the reference simple theme.

use super::{PipsPager, PipsPagerTemplateSettings};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{test_scope, TestRoot, TestScope};
use crate::testing::{
    add_pips_pager_themes, add_split_view_list_box_themes, create_test_theme, TestServices, UnitTestApplication,
    UnitTestApplicationScope,
};
use crate::{Button, ListBox, StackPanel};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::BindingMode;
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::Orientation;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::styling::{ControlTheme, Styles};
use ferroui_base::{AnyValue, FerroLocator, LocatorExtensions, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The test application and the text services it runs with.
struct TestApplication {
    // Dropped in this order: the application ends before the text services.
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

/// Starts the test application with the styled window services.
fn start() -> TestApplication {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::styled_window().with_render_interface(render_interface));
    TestApplication { _app: app, _text: text }
}

fn scoped() -> TestScope {
    test_scope()
}

fn pager(number_of_pages: i32, selected_page_index: i32) -> Ref<PipsPager> {
    let target = PipsPager::new();
    target.set_number_of_pages(number_of_pages);
    target.set_selected_page_index(selected_page_index);
    target
}

fn raise_key_down(target: &PipsPager, key: Key) {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(InputElement::key_down_event()));
    target.raise_event(&e);
}

fn find_button(target: &PipsPager, name: &str) -> Option<Ref<Button>> {
    target
        .get_visual_descendants()
        .filter_map(|v| v.cast::<Button>())
        .find(|b| b.name().as_deref() == Some(name))
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

fn get_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, scope| {
        let panel = StackPanel::new();

        let previous = Button::new();
        previous.set_name(Some("PART_PreviousButton".to_string()));
        panel.children().add(previous.register_in_name_scope(&**scope));

        let list = ListBox::new();
        list.set_name(Some("PART_PipsPagerList".to_string()));
        panel.children().add(list.register_in_name_scope(&**scope));

        let next = Button::new();
        next.set_name(Some("PART_NextButton".to_string()));
        panel.children().add(next.register_in_name_scope(&**scope));

        panel.upcast()
    })
}

#[test]
fn number_of_pages_should_update_pips() {
    let _scope = scoped();
    let target = PipsPager::new();

    target.set_number_of_pages(5);

    let pips = target.template_settings().pips();
    assert_eq!(5, pips.count());
    assert_eq!(1, pips.get(0));
    assert_eq!(5, pips.get(4));
}

#[test]
fn decreasing_number_of_pages_should_update_pips() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(5);

    target.set_number_of_pages(3);

    assert_eq!(3, target.template_settings().pips().count());
}

#[test]
fn decreasing_number_of_pages_should_update_selected_page_index() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(5);
    target.set_selected_page_index(4);

    target.set_number_of_pages(3);

    assert_eq!(2, target.selected_page_index());
}

#[test]
fn selected_page_index_should_be_clamped_to_zero() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(5);

    target.set_selected_page_index(-1);

    assert_eq!(0, target.selected_page_index());
}

#[test]
fn selected_page_index_change_should_raise_event() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(5);
    let raised = Rc::new(Cell::new(false));
    target.selected_index_changed({
        let raised = raised.clone();
        move |_, _| raised.set(true)
    });

    target.set_selected_page_index(2);

    assert!(raised.get());
}

#[test]
fn next_button_should_increment_index() {
    let _app = start();

    let target = pager(5, 1);
    target.set_is_next_button_visible(true);
    target.set_template(Some(get_template()));

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    let next_button = find_button(&target, "PART_NextButton");
    assert!(next_button.is_some());

    click(&next_button.unwrap());

    assert_eq!(2, target.selected_page_index());
}

#[test]
fn previous_button_should_decrement_index() {
    let _app = start();

    let target = pager(5, 3);
    target.set_is_previous_button_visible(true);
    target.set_template(Some(get_template()));

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    let prev_button = find_button(&target, "PART_PreviousButton");
    assert!(prev_button.is_some());

    click(&prev_button.unwrap());

    assert_eq!(2, target.selected_page_index());
}

#[test]
fn keyboard_navigation_should_work() {
    let _app = start();

    let target = pager(5, 1);
    target.set_orientation(Orientation::Horizontal);

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    raise_key_down(&target, Key::Right);
    assert_eq!(2, target.selected_page_index());

    raise_key_down(&target, Key::Left);
    assert_eq!(1, target.selected_page_index());

    target.set_orientation(Orientation::Vertical);

    raise_key_down(&target, Key::Down);
    assert_eq!(2, target.selected_page_index());

    raise_key_down(&target, Key::Up);
    assert_eq!(1, target.selected_page_index());
}

#[test]
fn orientation_pseudo_classes_should_be_set() {
    let _scope = scoped();
    let target = PipsPager::new();

    target.set_orientation(Orientation::Horizontal);
    assert!(target.classes().contains(":horizontal"));
    assert!(!target.classes().contains(":vertical"));

    target.set_orientation(Orientation::Vertical);
    assert!(!target.classes().contains(":horizontal"));
    assert!(target.classes().contains(":vertical"));
}

#[test]
fn clamping_logic_works() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(5);

    target.set_selected_page_index(10);
    assert_eq!(4, target.selected_page_index());

    target.set_selected_page_index(-5);
    assert_eq!(0, target.selected_page_index());
}

#[test]
fn manual_button_visibility_should_be_respected() {
    let _app = start();

    let target = PipsPager::new();
    target.set_number_of_pages(5);
    target.set_is_previous_button_visible(false);
    target.set_is_next_button_visible(false);
    target.set_template(Some(get_template()));

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    assert!(!target.is_previous_button_visible());
    assert!(!target.is_next_button_visible());

    target.set_is_previous_button_visible(true);
    target.set_is_next_button_visible(true);
    assert!(target.is_previous_button_visible());
    assert!(target.is_next_button_visible());
}

#[test]
fn simple_theme_should_forward_custom_button_themes() {
    let _app = start();

    let previous_theme = ControlTheme::with_target_type(Button::TYPE);
    let next_theme = ControlTheme::with_target_type(Button::TYPE);
    // The simple theme of the reference test.
    let simple_theme = Styles::new();
    simple_theme.add(create_test_theme());
    add_split_view_list_box_themes(&simple_theme);
    add_pips_pager_themes(&simple_theme);

    let target = PipsPager::new();
    target.set_number_of_pages(5);
    target.set_previous_button_theme(&previous_theme);
    target.set_next_button_theme(&next_theme);

    let root = TestRoot::new();
    root.set_child(target.clone());
    root.styles().add(ferroui_base::styling::styles_as_style(&simple_theme));

    let theme = simple_theme
        .resources()
        .try_get_resource(&ResourceKey::Type(PipsPager::TYPE), None)
        .flatten()
        .expect("the simple theme has a pips pager theme");
    let theme: &dyn AnyValue = &*theme;
    let theme = theme.downcast_ref::<Ref<ControlTheme>>().expect("the resource is a control theme").clone();
    target.set_theme(&theme);

    target.apply_template();
    root.layout_manager().execute_initial_layout_pass();

    let previous_button = find_button(&target, "PART_PreviousButton").unwrap();
    let next_button = find_button(&target, "PART_NextButton").unwrap();

    assert!(previous_button.theme().is_some_and(|t| t == previous_theme));
    assert!(next_button.theme().is_some_and(|t| t == next_theme));
}

#[test]
fn rapid_page_changes_should_maintain_integrity() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(100);
    let list = Rc::new(RefCell::new(Vec::new()));
    target.selected_index_changed({
        let list = list.clone();
        move |_, e| list.borrow_mut().push(e.new_index())
    });

    for i in 1..=50 {
        target.set_selected_page_index(i);
    }

    assert_eq!(50, list.borrow().len());
    assert_eq!(50, target.selected_page_index());
    assert_eq!(Some(&50), list.borrow().last());
}

#[test]
fn selected_index_changed_event_should_have_correct_args() {
    let _scope = scoped();
    let target = pager(5, 1);
    let old_idx = Rc::new(Cell::new(-1));
    let new_idx = Rc::new(Cell::new(-1));
    target.selected_index_changed({
        let (old_idx, new_idx) = (old_idx.clone(), new_idx.clone());
        move |_, e| {
            old_idx.set(e.old_index());
            new_idx.set(e.new_index());
        }
    });

    target.set_selected_page_index(3);
    assert_eq!(1, old_idx.get());
    assert_eq!(3, new_idx.get());
}

#[test]
fn pager_size_should_update_based_on_orientation_and_max_visible_pips() {
    let _app = start();

    let target = PipsPager::new();
    target.set_number_of_pages(10);
    target.set_max_visible_pips(5);
    target.set_orientation(Orientation::Horizontal);
    target.set_template(Some(get_template()));

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    let pips_list = target
        .get_visual_descendants()
        .filter_map(|v| v.cast::<ListBox>())
        .find(|i| i.name().as_deref() == Some("PART_PipsPagerList"))
        .unwrap();

    assert_eq!(60.0, pips_list.width());

    target.set_orientation(Orientation::Vertical);
    assert_eq!(60.0, pips_list.height());
}

#[test]
fn number_of_pages_reduction_should_clamp_selected_page_index() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(10);
    target.set_selected_page_index(8);

    target.set_number_of_pages(5);
    assert_eq!(4, target.selected_page_index());
}

#[test]
fn page_pseudo_classes_should_be_set() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(5);

    target.set_selected_page_index(0);
    assert!(target.classes().contains(":first-page"));
    assert!(!target.classes().contains(":last-page"));

    target.set_selected_page_index(2);
    assert!(!target.classes().contains(":first-page"));
    assert!(!target.classes().contains(":last-page"));

    target.set_selected_page_index(4);
    assert!(!target.classes().contains(":first-page"));
    assert!(target.classes().contains(":last-page"));
}

#[test]
fn navigation_buttons_is_enabled_should_update() {
    let _app = start();

    let target = PipsPager::new();
    target.set_number_of_pages(3);
    target.set_template(Some(get_template()));

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    let prev_button = find_button(&target, "PART_PreviousButton").unwrap();
    let next_button = find_button(&target, "PART_NextButton").unwrap();

    target.set_selected_page_index(0);
    assert!(!prev_button.is_enabled());
    assert!(next_button.is_enabled());

    target.set_selected_page_index(1);
    assert!(prev_button.is_enabled());
    assert!(next_button.is_enabled());

    target.set_selected_page_index(2);
    assert!(prev_button.is_enabled());
    assert!(!next_button.is_enabled());
}

#[test]
fn horizontal_keyboard_navigation_should_work() {
    let _scope = scoped();
    let target = pager(5, 1);
    target.set_orientation(Orientation::Horizontal);

    raise_key_down(&target, Key::Right);
    assert_eq!(2, target.selected_page_index());

    raise_key_down(&target, Key::Left);
    assert_eq!(1, target.selected_page_index());
}

#[test]
fn vertical_keyboard_navigation_should_work() {
    let _scope = scoped();
    let target = pager(5, 1);
    target.set_orientation(Orientation::Vertical);

    raise_key_down(&target, Key::Down);
    assert_eq!(2, target.selected_page_index());

    raise_key_down(&target, Key::Up);
    assert_eq!(1, target.selected_page_index());
}

#[test]
fn number_of_pages_zero_should_clamp_index() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(0);
    target.set_selected_page_index(5);

    assert_eq!(0, target.selected_page_index());
}

#[test]
fn home_key_should_navigate_to_first_page() {
    let _scope = scoped();
    let target = pager(10, 7);
    target.set_orientation(Orientation::Horizontal);

    raise_key_down(&target, Key::Home);

    assert_eq!(0, target.selected_page_index());
}

#[test]
fn end_key_should_navigate_to_last_page() {
    let _scope = scoped();
    let target = pager(10, 3);
    target.set_orientation(Orientation::Horizontal);

    raise_key_down(&target, Key::End);

    assert_eq!(9, target.selected_page_index());
}

#[test]
fn home_end_keys_should_work_in_vertical_orientation() {
    let _scope = scoped();
    let target = pager(10, 5);
    target.set_orientation(Orientation::Vertical);

    raise_key_down(&target, Key::Home);
    assert_eq!(0, target.selected_page_index());

    raise_key_down(&target, Key::End);
    assert_eq!(9, target.selected_page_index());
}

#[test]
fn negative_number_of_pages_should_be_coerced_to_zero() {
    let _scope = scoped();
    let target = PipsPager::new();

    target.set_number_of_pages(-5);

    assert_eq!(0, target.number_of_pages());
    assert_eq!(0, target.template_settings().pips().count());
}

#[test]
fn next_button_at_last_page_should_not_change_index() {
    let _app = start();

    let target = pager(3, 2);
    target.set_template(Some(get_template()));

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    let next_button = find_button(&target, "PART_NextButton").unwrap();
    click(&next_button);

    assert_eq!(2, target.selected_page_index());
}

#[test]
fn previous_button_at_first_page_should_not_change_index() {
    let _app = start();

    let target = pager(3, 0);
    target.set_template(Some(get_template()));

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    let prev_button = find_button(&target, "PART_PreviousButton").unwrap();
    click(&prev_button);

    assert_eq!(0, target.selected_page_index());
}

#[test]
fn arrow_keys_at_boundaries_should_not_change_index() {
    let _scope = scoped();
    let target = pager(5, 0);
    target.set_orientation(Orientation::Horizontal);

    raise_key_down(&target, Key::Left);
    assert_eq!(0, target.selected_page_index());

    target.set_selected_page_index(4);
    raise_key_down(&target, Key::Right);
    assert_eq!(4, target.selected_page_index());
}

#[test]
fn selected_page_index_default_binding_mode_should_be_two_way() {
    let _scope = scoped();
    assert_eq!(
        BindingMode::TwoWay,
        PipsPager::selected_page_index_property().get_metadata(PipsPager::TYPE).default_binding_mode()
    );
}

#[test]
fn template_settings_should_not_be_externally_settable() {
    let _scope = scoped();
    let target = PipsPager::new();

    // The template settings have no public setter (compile-time
    // enforcement). Verify the property is readable and initialized.
    let settings = target.template_settings();
    assert!(std::ptr::eq(settings.get_type(), PipsPagerTemplateSettings::TYPE));
}

#[test]
fn number_of_pages_to_zero_should_clamp_selected_page_index() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(5);
    target.set_selected_page_index(3);

    target.set_number_of_pages(0);

    assert_eq!(0, target.selected_page_index());
    assert_eq!(0, target.template_settings().pips().count());
}

#[test]
fn negative_number_of_pages_after_having_pages_should_coerce() {
    let _scope = scoped();
    let target = PipsPager::new();
    target.set_number_of_pages(5);
    assert_eq!(5, target.template_settings().pips().count());

    target.set_number_of_pages(-1);

    assert_eq!(0, target.number_of_pages());
    assert_eq!(0, target.template_settings().pips().count());
}

#[test]
fn preselected_index_should_be_preserved_after_template_apply() {
    let _app = start();

    let target = PipsPager::new();
    target.set_number_of_pages(20);
    target.set_max_visible_pips(5);
    target.set_selected_page_index(15);
    target.set_template(Some(get_template()));

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    assert_eq!(15, target.selected_page_index());
    assert!(!target.classes().contains(":last-page"));
    assert!(!target.classes().contains(":first-page"));
}

#[test]
fn preselected_last_index_should_set_last_page_pseudo_class() {
    let _scope = scoped();
    let target = pager(10, 9);

    assert_eq!(9, target.selected_page_index());
    assert!(target.classes().contains(":last-page"));
    assert!(!target.classes().contains(":first-page"));
}

// --- additional tests ---

/// Additional: the template of the test theme (the template of the
/// reference simple theme) shows one pip per page and binds the selected pip
/// both ways to the selected page.
#[test]
fn simple_theme_template_binds_the_pips_and_the_selected_page() {
    let _app = start();

    let simple_theme = Styles::new();
    simple_theme.add(create_test_theme());
    add_split_view_list_box_themes(&simple_theme);
    add_pips_pager_themes(&simple_theme);

    let target = pager(5, 2);

    let root = TestRoot::new();
    root.styles().add(ferroui_base::styling::styles_as_style(&simple_theme));
    root.set_child(target.clone());

    target.apply_template();
    root.layout_manager().execute_initial_layout_pass();

    let pips_list = target
        .get_visual_descendants()
        .filter_map(|v| v.cast::<ListBox>())
        .find(|i| i.name().as_deref() == Some("PART_PipsPagerList"))
        .expect("the template has the pips list");

    assert_eq!(5, pips_list.item_count());
    assert_eq!(2, pips_list.selected_index());

    target.set_selected_page_index(4);
    assert_eq!(4, pips_list.selected_index());

    pips_list.set_selected_index(1);
    assert_eq!(1, target.selected_page_index());

    // Five pips of 12 with a spacing of 4.
    assert_eq!(76.0, pips_list.width());
}

// An additional test (not a port): the automation names the template parts get.
#[test]
fn additional_navigation_buttons_get_their_automation_names() {
    let _app = start();

    let target = pager(5, 1);
    target.set_is_previous_button_visible(true);
    target.set_is_next_button_visible(true);
    target.set_template(Some(get_template()));

    let _root = TestRoot::with_child(target.clone());
    target.apply_template();

    let previous_button = find_button(&target, "PART_PreviousButton").unwrap();
    let next_button = find_button(&target, "PART_NextButton").unwrap();
    assert_eq!(Some("Previous page".to_string()), crate::automation::AutomationProperties::get_name(&previous_button));
    assert_eq!(Some("Next page".to_string()), crate::automation::AutomationProperties::get_name(&next_button));
}
