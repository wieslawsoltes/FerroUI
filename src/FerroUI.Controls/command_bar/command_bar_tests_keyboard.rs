//! Port of the reference `CommandBarTests`: the test class of the keyboard
//! of the overflow menu (`CommandBarOverflowKeyboardTests`).
//!
//! Adapted:
//!
//! - The reference gives the test roots the styles of the application as
//!   their styling parent; the test root of this crate has no styling
//!   parent, so the theme of the application is added to the styles of the
//!   root instead.

use super::{CommandBar, CommandBarButton, CommandBarSeparator, ICommandBarElement};
use crate::primitives::{Popup, TemplatedControlImpl};
use crate::test_support::TestRoot;
use crate::testing::{create_test_theme, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Button, ButtonImpl, ContentControlImpl, Control, ControlImpl, ItemsControl, Window};
use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers, NavigationMethod, Pointer,
    PointerPointProperties, PointerPressedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::{Interactive, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs,
    Point, Ref, StyledElement, StyledElementImpl, Visual, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

/// The running application of a test (the application the test class of
/// the reference starts in its constructor), with the text services of the
/// tests registered over its services.
struct AppScope {
    // Dropped in this order: the text services first.
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

fn start() -> AppScope {
    let app = UnitTestApplication::start(TestServices::focusable_window());
    AppScope { _text: TextTestScope::new(), _app: app }
}

fn button(label: &str) -> Ref<CommandBarButton> {
    let button = CommandBarButton::new();
    button.set_label(Some(label));
    button
}

/// A test root styled with the theme of the application, with `child` as
/// its child.
fn test_root(child: &Ref<CommandBar>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.styles().add(create_test_theme());
    root.set_child(child.clone());
    root
}

fn get_overflow_presenter(cb: &CommandBar) -> Ref<ItemsControl> {
    let popup = cb
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Popup>())
        .find(|popup| popup.name().as_deref() == Some("PART_OverflowPopup"))
        .expect("the command bar template has an overflow popup");

    popup
        .get_logical_descendants()
        .filter_map(|element| element.cast::<ItemsControl>())
        .find(|items_control| items_control.name().as_deref() == Some("PART_OverflowPresenter"))
        .expect("the overflow popup has an overflow presenter")
}

fn key_down(key: Key) -> KeyEventArgs {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    args
}

fn raise_key_on_overflow_presenter(cb: &CommandBar, key: Key) {
    get_overflow_presenter(cb).raise_event(&key_down(key));
}

fn get_overflow_button(cb: &CommandBar) -> Ref<Button> {
    cb.get_visual_descendants()
        .filter_map(|visual| visual.cast::<Button>())
        .find(|button| button.name().as_deref() == Some("PART_OverflowButton"))
        .expect("the command bar template has an overflow button")
}

fn create_window(content: &Ref<CommandBar>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(content.clone())));
    window.show();
    window.apply_styling();
    window.apply_template();
    window
}

fn raise_pointer_pressed(target: &Ref<Button>) {
    let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let visual: Ref<Visual> = target.clone().upcast();
    target.raise_event(&PointerPressedEventArgs::new(
        target.clone().upcast::<Interactive>(),
        pointer,
        &visual,
        Point::default(),
        1,
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    ));
}

/// A command bar button that hides itself when it is first given a parent.
#[repr(C)]
struct VisibilityChangingCommandBarButton {
    base: CommandBarButton,
    has_updated_visibility: Cell<bool>,
}

ferro_class!(VisibilityChangingCommandBarButton: CommandBarButton);
ferro_impl_classes!(
    VisibilityChangingCommandBarButton: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl
);

impl FerroObjectImpl for VisibilityChangingCommandBarButton {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if !this.has_updated_visibility.get()
            && change.property() == StyledElement::parent_property().as_property()
            && change.get_new_value::<Option<Ref<StyledElement>>>().is_some()
        {
            this.has_updated_visibility.set(true);
            this.set_current_value(Visual::is_visible_property(), false);
        }
    }
}

impl VisibilityChangingCommandBarButton {
    fn new() -> Ref<Self> {
        instantiate(Self { base: CommandBarButton::construct(), has_updated_visibility: Cell::new(false) })
    }
}

fn el(button: &Ref<CommandBarButton>) -> Rc<dyn ICommandBarElement> {
    button.as_command_bar_element()
}

#[test]
fn escape_when_overflow_open_closes_overflow() {
    let _app = start();
    let cb = CommandBar::new();
    cb.secondary_commands().add(el(&button("Action")));
    let root = test_root(&cb);
    root.layout_manager().execute_initial_layout_pass();
    cb.set_is_open(true);

    raise_key_on_overflow_presenter(&cb, Key::Escape);

    assert!(!cb.is_open());
}

#[test]
fn escape_when_overflow_closed_does_nothing() {
    let _app = start();
    let cb = CommandBar::new();
    cb.secondary_commands().add(el(&button("Action")));
    let root = test_root(&cb);
    root.layout_manager().execute_initial_layout_pass();

    raise_key_on_overflow_presenter(&cb, Key::Escape);

    assert!(!cb.is_open());
}

#[test]
fn navigation_keys_are_handled_when_overflow_has_items() {
    for key in [Key::Down, Key::Up, Key::Home, Key::End] {
        let _app = start();
        let cb = CommandBar::new();
        cb.secondary_commands().add(el(&button("A")));
        cb.secondary_commands().add(el(&button("B")));
        let root = test_root(&cb);
        root.layout_manager().execute_initial_layout_pass();
        cb.set_is_open(true);

        let e = key_down(key);
        get_overflow_presenter(&cb).raise_event(&e);

        assert!(e.handled(), "{key:?}");
    }
}

#[test]
fn navigation_keys_are_handled_when_all_items_are_separators() {
    let _app = start();
    let cb = CommandBar::new();
    cb.secondary_commands().add(CommandBarSeparator::new().as_command_bar_element());
    let root = test_root(&cb);
    root.layout_manager().execute_initial_layout_pass();
    cb.set_is_open(true);

    let e = key_down(Key::Down);
    get_overflow_presenter(&cb).raise_event(&e);

    assert!(e.handled());
}

#[test]
fn navigation_keys_are_handled_when_all_items_are_disabled() {
    let _app = start();
    let cb = CommandBar::new();
    let disabled = button("A");
    disabled.set_is_enabled(false);
    cb.secondary_commands().add(el(&disabled));
    let root = test_root(&cb);
    root.layout_manager().execute_initial_layout_pass();
    cb.set_is_open(true);

    let e = key_down(Key::Down);
    get_overflow_presenter(&cb).raise_event(&e);

    assert!(e.handled());
}

#[test]
fn navigation_keys_are_handled_when_all_items_are_non_focusable() {
    let _app = start();
    let cb = CommandBar::new();
    let non_focusable = button("A");
    non_focusable.set_focusable(false);
    cb.secondary_commands().add(el(&non_focusable));
    let root = test_root(&cb);
    root.layout_manager().execute_initial_layout_pass();
    cb.set_is_open(true);

    let e = key_down(Key::Down);
    get_overflow_presenter(&cb).raise_event(&e);

    assert!(e.handled());
}

#[test]
fn keyboard_open_focuses_first_overflow_item_as_focus_visible() {
    let _app = start();
    let first = button("A");
    let cb = CommandBar::new();
    cb.secondary_commands().add(el(&first));
    let _window = create_window(&cb);

    assert!(get_overflow_button(&cb).focus_with(NavigationMethod::Tab, KeyModifiers::NONE));

    cb.set_is_open(true);
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));

    assert!(first.is_focused());
    assert!(first.classes().contains(":focus-visible"));
}

#[test]
fn pointer_open_after_keyboard_focus_does_not_make_first_overflow_item_focus_visible() {
    let _app = start();
    let first = button("A");
    let cb = CommandBar::new();
    cb.secondary_commands().add(el(&first));
    let _window = create_window(&cb);

    let overflow_button = get_overflow_button(&cb);
    assert!(overflow_button.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
    raise_pointer_pressed(&overflow_button);

    cb.set_is_open(true);
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));

    assert!(first.is_focused());
    assert!(!first.classes().contains(":focus-visible"));
}

#[test]
fn keyboard_open_after_pointer_focus_makes_first_overflow_item_focus_visible() {
    let _app = start();
    let first = button("A");
    let cb = CommandBar::new();
    cb.secondary_commands().add(el(&first));
    let _window = create_window(&cb);

    let overflow_button = get_overflow_button(&cb);
    assert!(overflow_button.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE));
    overflow_button.raise_event(&key_down(Key::Space));

    cb.set_is_open(true);
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));

    assert!(first.is_focused());
    assert!(first.classes().contains(":focus-visible"));
}

#[test]
fn open_does_not_throw_when_secondary_command_visibility_changes_during_overflow_realization() {
    let _app = start();
    let secondary = VisibilityChangingCommandBarButton::new();
    secondary.set_label(Some("Bound"));
    let cb = CommandBar::new();
    cb.secondary_commands().add(secondary.as_command_bar_element());
    let _window = create_window(&cb);

    cb.set_is_open(true);

    assert!(cb.is_open());
    assert!(!secondary.is_visible());
}
