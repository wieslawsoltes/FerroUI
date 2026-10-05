//! Port of the reference `ContextMenuTests`.
//!
//! The reference shares one popup implementation mock between all the
//! popups of a test and verifies the calls made on it. Here the window
//! implementation creates a popup mock per popup and the tests count the
//! calls over all of them.
//!
//! The two tests the reference builds from markup
//! (`context_menu_in_resources_can_be_shared`,
//! `context_menu_can_be_set_in_style`) build the same tree in code.

use crate::mouse_test_helper::MouseTestHelper;
use crate::platform::{IPopupImpl, IScreenImpl, ITopLevelImpl, IWindowImpl};
use crate::primitives::{LightDismissOverlayLayer, Popup};
use crate::testing::{
    mock_screen, MockCall, MockScreenImpl, MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
    UnitTestApplicationScope,
};
use crate::{
    Button, ContentControl, ContextMenu, Control, Flyout, MenuItem, Panel, StackPanel, TextBlock, Window,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::{
    ContextRequestedEventArgs, IKeyboardDevice, InputElement, InputManager, Key, KeyEventArgs, KeyModifiers,
    KeyboardDevice, MouseButton,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::styling::{ISetterValue, Selectors, Setter, Style};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{AnyValue, BoxedValue, FerroLocator, LocatorExtensions, PixelPoint, PixelRect, PixelSize, Point, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The application of a test and the popup implementations it created.
struct TestApplication {
    popup_impls: Rc<RefCell<Vec<Rc<MockWindowImpl>>>>,
    mouse: MouseTestHelper,
    _app: UnitTestApplicationScope,
    _text: Option<TextTestScope>,
}

impl TestApplication {
    /// The number of `show(true, false)` calls made on the popup
    /// implementations.
    fn show_count(&self) -> usize {
        let call = MockCall::Show { activate: true, is_dialog: false };
        self.popup_impls.borrow().iter().map(|popup_impl| popup_impl.count_of(&call)).sum()
    }

    /// The number of `hide()` calls made on the popup implementations.
    fn hide_count(&self) -> usize {
        self.popup_impls.borrow().iter().map(|popup_impl| popup_impl.count_of(&MockCall::Hide)).sum()
    }
}

fn application() -> TestApplication {
    application_with(false)
}

/// The application of the tests whose controls lay out text (the reference
/// services of every test can): the test fonts are installed as well.
fn application_with_text() -> TestApplication {
    application_with(true)
}

fn application_with(text: bool) -> TestApplication {
    let screen = PixelRect::from_position_size(PixelPoint::default(), PixelSize::new(100, 100));
    let screen_impl: Rc<dyn IScreenImpl> = MockScreenImpl::new(vec![mock_screen(1.0, screen, screen, true)]);

    let popup_impls: Rc<RefCell<Vec<Rc<MockWindowImpl>>>> = Rc::new(RefCell::new(Vec::new()));

    let create_window_impl = {
        let popup_impls = popup_impls.clone();
        move || -> Rc<dyn IWindowImpl> {
            let window_impl = MockWindowingPlatform::create_window_mock();
            let weak = Rc::downgrade(&window_impl);
            let popup_impls = popup_impls.clone();
            window_impl.setup_create_popup(move |_| {
                let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
                let popup_impl = MockWindowingPlatform::create_popup_mock(parent);
                popup_impl.render_scaling.set(1.0);
                popup_impls.borrow_mut().push(popup_impl.clone());
                Some(popup_impl as Rc<dyn IPopupImpl>)
            });
            window_impl.setup_feature::<dyn IScreenImpl>(screen_impl.clone());
            window_impl
        }
    };

    let text = text.then(TextTestScope::new);
    let mut services = TestServices::styled_window();
    if text.is_some() {
        let render_interface = FerroLocator::current()
            .get_service::<dyn IPlatformRenderInterface>()
            .expect("the text services have a render interface");
        services = services.with_render_interface(render_interface);
    }
    let services = services
        .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>))
        .with_input_manager(Rc::new(InputManager::new()))
        .with_windowing_platform(MockWindowingPlatform::with_window_impl(create_window_impl));

    TestApplication {
        popup_impls,
        mouse: MouseTestHelper::new(),
        _app: UnitTestApplication::start(services),
        _text: text,
    }
}

/// A window with the given content whose styling and templates are applied
/// (the reference tests do so with or without their `PreparedWindow`
/// helper, which only adds a compositor to the window implementation).
fn prepared_window(content: Option<Ref<Control>>) -> Ref<Window> {
    let w = Window::new();
    w.set_content(content.map(Control::boxed));
    w.apply_styling();
    w.apply_template();
    w.presenter().expect("the window has a presenter").apply_template();
    w
}

fn create_key_up_event_args(key: Key, source: &Ref<Window>) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(InputElement::key_up_event()));
    e.set_source(source);
    e
}

/// A panel with the given context menu.
fn panel_with(context_menu: &Ref<ContextMenu>) -> Ref<Panel> {
    let target = Panel::new();
    target.set_context_menu(context_menu);
    target
}

fn small_top_left_button() -> Ref<Button> {
    let button = Button::new();
    button.set_height(10.0);
    button.set_width(10.0);
    button.set_horizontal_alignment(HorizontalAlignment::Left);
    button.set_vertical_alignment(VerticalAlignment::Top);
    button
}

fn right_click(mouse: &MouseTestHelper, target: &Control) {
    mouse.click_with(target, target, MouseButton::Right, None, KeyModifiers::NONE);
}

fn down(mouse: &MouseTestHelper, target: &Control, button: MouseButton) {
    mouse.down_with(target, target, button, None, KeyModifiers::NONE, 1);
}

fn up(mouse: &MouseTestHelper, target: &Control, button: MouseButton) {
    mouse.up_with(target, target, button, None, KeyModifiers::NONE);
}

fn light_dismiss_overlay(window: &Ref<Window>) -> Ref<LightDismissOverlayLayer> {
    LightDismissOverlayLayer::get_light_dismiss_overlay_layer(window)
        .expect("the window has a light dismiss overlay layer")
}

#[test]
fn context_requested_opens_context_menu() {
    let _app = application();
    let sut = ContextMenu::new();
    let target = panel_with(&sut);

    let _window = prepared_window(Some(target.clone().upcast()));

    let opened_count = Rc::new(Cell::new(0));

    let count = opened_count.clone();
    sut.opened(move |_, _| count.set(count.get() + 1));

    target.raise_event(&ContextRequestedEventArgs::new());

    assert!(sut.is_open());
    assert_eq!(1, opened_count.get());
}

#[test]
fn context_menu_is_opened_when_context_flyout_is_also_set() {
    // We have this test for backwards compatability with the code that already sets custom ContextMenu.
    let _app = application();
    let sut = ContextMenu::new();
    let flyout = Flyout::new();
    let target = Panel::new();
    target.set_context_menu(&sut);
    target.set_context_flyout(&flyout);

    let _window = prepared_window(Some(target.clone().upcast()));

    target.raise_event(&ContextRequestedEventArgs::new());

    assert!(sut.is_open());
    assert!(!flyout.is_open());
}

#[test]
fn key_up_raised_on_target_opens_context_flyout() {
    let _app = application();
    let sut = ContextMenu::new();
    let target = panel_with(&sut);
    let context_requested_count = Rc::new(Cell::new(0));
    let count = context_requested_count.clone();
    target.add_handler_with(
        InputElement::context_requested_event(),
        move |_, _: &ContextRequestedEventArgs| count.set(count.get() + 1),
        RoutingStrategies::TUNNEL,
        false,
    );

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    target.raise_event(&create_key_up_event_args(Key::Apps, &window));

    assert!(sut.is_open());
    assert_eq!(1, context_requested_count.get());
}

#[test]
fn key_up_raised_on_flyout_closes_opened_context_menu() {
    let _app = application();
    let sut = ContextMenu::new();
    let target = panel_with(&sut);

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    target.raise_event(&ContextRequestedEventArgs::new());

    assert!(sut.is_open());

    sut.raise_event(&create_key_up_event_args(Key::Apps, &window));

    assert!(!sut.is_open());
}

#[test]
fn opening_raises_single_opened_event() {
    let _app = application();
    let sut = ContextMenu::new();
    let target = panel_with(&sut);

    let _window = prepared_window(Some(target.clone().upcast()));

    let opened_count = Rc::new(Cell::new(0));

    let count = opened_count.clone();
    sut.opened(move |_, _| count.set(count.get() + 1));

    sut.open_at(Some(&target.clone().upcast()));

    assert_eq!(1, opened_count.get());
}

#[test]
fn open_should_use_default_control() {
    let _app = application();
    let sut = ContextMenu::new();
    let target = panel_with(&sut);

    let _window = prepared_window(Some(target.clone().upcast()));

    let opened = Rc::new(Cell::new(false));

    let flag = opened.clone();
    sut.opened(move |_, _| flag.set(true));

    sut.open();

    assert!(opened.get());
}

#[test]
fn open_should_raise_exception_if_already_detached() {
    let _app = application();
    let sut = ContextMenu::new();
    let target = panel_with(&sut);

    let _window = prepared_window(Some(target.clone().upcast()));

    target.set_context_menu(None);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sut.open()));
    assert!(result.is_err());
}

#[test]
fn closing_raises_single_closed_event() {
    let _app = application();
    let sut = ContextMenu::new();
    let target = panel_with(&sut);

    let _window = prepared_window(Some(target.clone().upcast()));

    sut.open_at(Some(&target.clone().upcast()));

    let closed_count = Rc::new(Cell::new(0));

    let count = closed_count.clone();
    sut.closed(move |_, _| count.set(count.get() + 1));

    sut.close();

    assert_eq!(1, closed_count.get());
}

#[test]
fn cancel_light_dismiss_closing_keeps_flyout_open() {
    let app = application();

    let window = prepared_window(None);
    window.set_width(100.0);
    window.set_height(100.0);

    let button = small_top_left_button();
    window.set_content(Some(Control::boxed(button.clone())));

    window.apply_template();
    window.show();

    let tracker = Rc::new(Cell::new(0));

    let c = ContextMenu::new();
    let count = tracker.clone();
    let _ = c.closing(move |e| {
        count.set(count.get() + 1);
        e.set_cancel(true);
    });
    button.set_context_menu(&c);
    c.open_at(Some(&button.clone().upcast()));

    let overlay = light_dismiss_overlay(&window);
    app.mouse.down_at(&overlay, MouseButton::Left, Point::new(90.0, 90.0), 1);
    app.mouse.up_at(&button, MouseButton::Left, Point::new(90.0, 90.0));

    assert_eq!(1, tracker.get());
    assert!(c.is_open());

    assert_eq!(0, app.hide_count());
    assert_eq!(1, app.show_count());
}

#[test]
fn light_dismiss_closes_flyout() {
    let app = application();

    let window = prepared_window(None);
    window.set_width(100.0);
    window.set_height(100.0);

    let button = small_top_left_button();
    window.set_content(Some(Control::boxed(button.clone())));

    window.apply_template();
    window.show();

    let c = ContextMenu::new();
    c.set_placement(crate::PlacementMode::Bottom);
    c.open_at(Some(&button.clone().upcast()));

    let overlay = light_dismiss_overlay(&window);
    app.mouse.down_at(&overlay, MouseButton::Left, Point::new(90.0, 90.0), 1);
    app.mouse.up_at(&button, MouseButton::Left, Point::new(90.0, 90.0));

    assert!(!c.is_open());
    assert_eq!(1, app.hide_count());
    assert_eq!(1, app.show_count());
}

#[test]
fn clicking_on_control_toggles_context_menu() {
    let app = application();

    let sut = ContextMenu::new();
    let target = panel_with(&sut);

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();
    let overlay = light_dismiss_overlay(&window);

    right_click(&app.mouse, &target);

    assert!(sut.is_open());

    app.mouse.down(&overlay);
    app.mouse.up(&target);

    assert!(!sut.is_open());
    assert_eq!(1, app.show_count());
    assert_eq!(1, app.hide_count());
}

#[test]
fn right_clicking_on_control_twice_re_opens_context_menu() {
    let app = application();

    let sut = ContextMenu::new();
    let target = panel_with(&sut);

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    let overlay = light_dismiss_overlay(&window);

    right_click(&app.mouse, &target);
    assert!(sut.is_open());

    down(&app.mouse, &overlay, MouseButton::Right);
    up(&app.mouse, &target, MouseButton::Right);

    assert!(sut.is_open());
    assert_eq!(1, app.hide_count());
    assert_eq!(2, app.show_count());
}

#[test]
fn context_menu_can_be_shared_between_controls_even_after_a_control_is_removed_from_visual_tree() {
    let app = application();
    let sut = ContextMenu::new();
    let target1 = panel_with(&sut);

    let target2 = panel_with(&sut);

    let sp = StackPanel::new();
    sp.children().add(target1.clone());
    sp.children().add(target2.clone());
    let _window = prepared_window(Some(sp.clone().upcast()));

    right_click(&app.mouse, &target1);

    assert!(sut.is_open());

    sp.children().remove(target1.clone());

    assert!(!sut.is_open());

    right_click(&app.mouse, &target2);

    assert!(sut.is_open());
}

#[test]
fn cancelling_opening_does_not_show_context_menu() {
    let app = application();

    let event_called = Rc::new(Cell::new(false));
    let sut = ContextMenu::new();
    let target = panel_with(&sut);
    let window = Window::new();
    window.set_content(Some(Control::boxed(target.clone())));

    let called = event_called.clone();
    let _ = sut.opening(move |e| {
        called.set(true);
        e.set_cancel(true);
    });

    right_click(&app.mouse, &target);

    assert!(event_called.get());
    assert!(!sut.is_open());
    assert_eq!(0, app.show_count());
}

#[test]
fn can_set_clear_context_menu_property() {
    let _app = application();
    let target = ContextMenu::new();
    let control = Panel::new();

    control.set_context_menu(&target);
    control.set_context_menu(None);
}

/// A content control stands in for the user control of the reference test.
#[test]
fn should_reset_popup_parent_on_target_detached() {
    let _app = application();
    let user_control = ContentControl::new();
    let window = prepared_window(Some(user_control.clone().upcast()));
    window.show();

    let menu = ContextMenu::new();
    user_control.set_context_menu(&menu);
    menu.open();

    let popup = menu.parent().and_then(|parent| parent.cast::<Popup>()).expect("the parent of the menu is a popup");
    assert!(popup.parent().is_some());

    window.set_content(None);
    assert!(popup.parent().is_none());
}

/// The two text blocks under a stack panel in a window, as in the markup
/// of the reference tests.
fn window_with_two_targets() -> (Ref<Window>, Ref<TextBlock>, Ref<TextBlock>) {
    let target1 = TextBlock::new();
    target1.set_name(Some("target1".to_string()));
    let target2 = TextBlock::new();
    target2.set_name(Some("target2".to_string()));

    let sp = StackPanel::new();
    sp.children().add(target1.clone());
    sp.children().add(target2.clone());

    let window = Window::new();
    window.set_content(Some(Control::boxed(sp)));
    (window, target1, target2)
}

/// A context menu with one item whose header is the text `Foo`.
fn context_menu_with_item() -> Ref<ContextMenu> {
    let item = MenuItem::new();
    item.set_header(Some(Rc::new("Foo".to_string()) as BoxedValue));
    let menu = ContextMenu::new();
    menu.items().add(Some(Control::boxed(item)));
    menu
}

fn assert_shared_menu_opens_on_both(app: &TestApplication, window: &Ref<Window>, target1: &TextBlock, target2: &TextBlock) {
    assert!(target1.context_menu().is_some());
    assert!(target2.context_menu().is_some());
    assert_eq!(target1.context_menu(), target2.context_menu());

    window.show();

    let menu = target1.context_menu().unwrap();
    right_click(&app.mouse, target1);
    assert!(menu.is_open());
    right_click(&app.mouse, target2);
    assert!(menu.is_open());
}

#[test]
fn context_menu_in_resources_can_be_shared() {
    let app = application_with_text();

    let (window, target1, target2) = window_with_two_targets();
    let key = ResourceKey::from("contextMenu");
    window.resources().add_value(key.clone(), context_menu_with_item());

    // The static resource references of the markup.
    for target in [&target1, &target2] {
        let value = window.resources().try_get_resource(&key, None).flatten().expect("the resource exists");
        let value: &dyn AnyValue = &*value;
        let menu = value.downcast_ref::<Ref<ContextMenu>>().expect("the resource is a context menu").clone();
        target.set_context_menu(&menu);
    }

    assert_shared_menu_opens_on_both(&app, &window, &target1, &target2);
}

#[test]
fn context_menu_can_be_set_in_style() {
    let app = application_with_text();

    let (window, target1, target2) = window_with_two_targets();

    // A context menu is accepted as the value of a setter of the
    // `ContextMenu` property, as a control (the way a control is seen by
    // untyped code) and as itself; any other property rejects it.
    let menu = context_menu_with_item();
    let value = Some(menu.clone());
    let setter = Setter::empty();
    setter.set_property(Some(Control::context_menu_property().as_property()));
    let as_control: &Control = &menu;
    setter.set_setter_value(as_control as &dyn ISetterValue, Rc::new(value.clone()));
    setter.set_setter_value(&*menu as &dyn ISetterValue, Rc::new(value));

    let other = Setter::empty();
    other.set_property(Some(Control::tag_property().as_property()));
    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        other.set_setter_value(as_control as &dyn ISetterValue, Rc::new(Some(menu.clone())));
    }));
    assert!(rejected.is_err());

    window.styles().add(Style::with_setters(Selectors::of_type::<TextBlock>(), [setter]));
    window.apply_styling();
    target1.apply_styling();
    target2.apply_styling();

    assert_shared_menu_opens_on_both(&app, &window, &target1, &target2);
}

#[test]
fn cancelling_closing_leaves_context_menu_open() {
    let app = application();

    let event_called = Rc::new(Cell::new(false));
    let sut = ContextMenu::new();
    let target = panel_with(&sut);

    let window = prepared_window(Some(target.clone().upcast()));
    let overlay = light_dismiss_overlay(&window);

    let called = event_called.clone();
    let _ = sut.closing(move |e| {
        called.set(true);
        e.set_cancel(true);
    });

    window.show();

    right_click(&app.mouse, &target);

    assert!(sut.is_open());

    down(&app.mouse, &overlay, MouseButton::Right);
    up(&app.mouse, &target, MouseButton::Right);

    assert!(event_called.get());
    assert!(sut.is_open());

    assert_eq!(1, app.show_count());
    assert_eq!(0, app.hide_count());
}

#[test]
fn closing_should_restore_focus() {
    let app = application();

    let item = MenuItem::new();
    let sut = ContextMenu::new();
    sut.items().add(Some(Control::boxed(item.clone())));

    let button = Button::new();
    let target = Panel::new();
    target.children().add(button.clone());
    target.set_context_menu(&sut);

    let window = prepared_window(Some(target.clone().upcast()));
    let focus_manager = window.focus_manager();

    // Show the window and focus the button.
    window.show();
    button.focus();
    assert_eq!(Some(button.clone().upcast::<InputElement>()), focus_manager.get_focused_element());

    // Click to show the context menu.
    right_click(&app.mouse, &target);
    assert!(sut.is_open());

    // Hover over the context menu item: this should focus it.
    app.mouse.enter(&item);
    assert_eq!(Some(item.clone().upcast::<InputElement>()), focus_manager.get_focused_element());

    // Click the menu item to close the menu.
    app.mouse.click(&item);
    assert!(!sut.is_open());

    // Focus should be restored to the button.
    assert_eq!(Some(button.upcast::<InputElement>()), focus_manager.get_focused_element());
}

// Not in the reference suite: the menu owns its popup and the popup shows
// the menu, so the popup lets go of the menu when it closes or the two
// would keep each other alive.
#[test]
fn closed_context_menu_is_released_with_its_owner() {
    let _app = application();
    let sut = ContextMenu::new();
    let button = Button::new();
    let target = panel_with(&sut);
    target.children().add(button.clone());

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    sut.open_at(Some(&target.clone().upcast()));
    assert!(sut.parent().is_some_and(|parent| parent.is::<Popup>()));
    sut.close();
    assert!(!sut.is_open());
    assert!(sut.parent().is_none());

    // The focus system holds the element that had the focus last: the menu,
    // until the focus moves on.
    button.focus();
    assert!(button.is_focused());

    target.set_context_menu(None);

    let weak = sut.downgrade();
    drop(sut);
    assert!(weak.upgrade().is_none());
}
