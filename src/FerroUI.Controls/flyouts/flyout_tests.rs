//! Port of the reference `FlyoutTests`.
//!
//! The reference runs the suite twice: with the platform creating native
//! popups (`FlyoutTests`) and with the platform creating none, so that
//! flyouts are shown in the overlay layer (`OverlayPopupFlyoutTests`, whose
//! `UseOverlayPopups` flag is true). Every test body here takes that flag
//! and is run by both variants.
//!
//! The `TestFlyout` of the reference only makes the popup of the flyout
//! public; it is public here, so the tests use [`Flyout`] itself.

use super::{Flyout, FlyoutBase, FlyoutPresenter, FlyoutShowMode, MenuFlyout};
use crate::platform::{IPopupImpl, ITopLevelImpl, IWindowImpl};
use crate::primitives::{LightDismissOverlayLayer, TemplatedControl};
use crate::mouse_test_helper::MouseTestHelper;
use crate::platform::{FeedbackAction, FeedbackType, IPlatformFeedback, PlatformFeedback};
use crate::testing::{
    MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use crate::test_support::boxed_str;
use crate::{Border, Button, ClickMode, ContentControl, Control, MenuItem, Panel, StackPanel, TextBlock, Window};
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingMode, ReflectionBinding};
use ferroui_base::input::{
    ContextRequestedEventArgs, IKeyboardDevice, InputElement, Key, KeyEventArgs, KeyModifiers, KeyboardDevice,
    MouseButton, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Brushes, Colors, Geometry, GeometryHitTestResult, IBrush};
use ferroui_base::rendering::IHitTester;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ferro_model, BoxedValue, Point, Ref, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn create_services_with_focus(use_overlay_popups: bool) -> UnitTestApplicationScope {
    UnitTestApplication::start(
        TestServices::styled_window()
            .with_windowing_platform(MockWindowingPlatform::with_window_impl(move || {
                create_window_impl(use_overlay_popups)
            }))
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
    )
}

fn create_window_impl(use_overlay_popups: bool) -> Rc<dyn IWindowImpl> {
    create_window_mock(use_overlay_popups)
}

fn create_window_mock(use_overlay_popups: bool) -> Rc<MockWindowImpl> {
    let mock = MockWindowingPlatform::create_window_mock();

    let weak = Rc::downgrade(&mock);
    mock.setup_create_popup(move |_| {
        if use_overlay_popups {
            return None;
        }
        let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
        Some(MockWindowingPlatform::create_popup_mock(parent) as Rc<dyn IPopupImpl>)
    });

    mock
}

fn prepared_window(content: Option<Ref<Control>>) -> Ref<Window> {
    let w = Window::new();
    w.set_content(content.map(Control::boxed));
    w.apply_template();
    w
}

fn create_pointer_pressed_event_args(source: &Ref<Window>, p: Point) -> PointerPressedEventArgs {
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    PointerPressedEventArgs::new(
        source.clone(),
        pointer,
        source,
        p,
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    )
}

fn create_key_up_event_args(key: Key, source: &Ref<Window>) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(InputElement::key_up_event()));
    e.set_source(source);
    e
}

/// A counter shared with an event handler.
fn tracker() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

/// A focusable control: stands in for the text boxes of the reference
/// tests, which only use them as focusable elements.
fn focusable() -> Ref<Border> {
    let border = Border::new();
    border.set_focusable(true);
    border
}

fn sized_border() -> Ref<Border> {
    let border = Border::new();
    border.set_width(10.0);
    border.set_height(10.0);
    border
}

/// A hit tester that answers `result` for the first visual at `point`
/// under `root` and nothing otherwise.
struct TestHitTester {
    point: Point,
    root: RefCell<Option<Ref<Visual>>>,
    result: RefCell<Option<Ref<Visual>>>,
}

impl IHitTester for TestHitTester {
    fn hit_test(&self, _p: Point, _root: &Visual, _filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        Vec::new()
    }

    fn hit_test_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        Vec::new()
    }

    fn hit_test_first(
        &self,
        p: Point,
        root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<Ref<Visual>> {
        let expected_root = self.root.borrow().clone();
        if p == self.point && expected_root.is_some_and(|expected_root| expected_root == root.to_ref()) {
            return self.result.borrow().clone();
        }
        None
    }

    fn hit_test_first_geometry(
        &self,
        _geometry: &Geometry,
        _root: &Visual,
        _filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        None
    }
}

struct FlyoutViewModel {
    is_open: Cell<bool>,
    property_changed: Event<str>,
}

impl FlyoutViewModel {
    fn new() -> Rc<Self> {
        Model::new_model(Self { is_open: Cell::new(false), property_changed: Event::new() })
    }

    fn is_open(&self) -> bool {
        self.is_open.get()
    }

    fn set_is_open(&self, value: bool) {
        if self.is_open.get() != value {
            self.is_open.set(value);
            self.property_changed.raise("IsOpen");
        }
    }
}

impl INotifyPropertyChanged for FlyoutViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(FlyoutViewModel, |b| b
    .notify_property_changed()
    .property::<Value<bool>>("IsOpen", |vm| vm.is_open(), |vm, v| vm.set_is_open(v)));

fn opening_raises_single_opening_event(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let tracker = tracker();
    let f = Flyout::new();
    let count = tracker.clone();
    let _ = f.opening(move |_| count.set(count.get() + 1));
    f.show_at(&window);

    assert_eq!(1, tracker.get());
    assert!(f.is_open());
}

fn opening_raises_single_opened_event(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let tracker = tracker();
    let f = Flyout::new();
    let count = tracker.clone();
    let _ = f.opened(move || count.set(count.get() + 1));
    f.show_at(&window);

    assert_eq!(1, tracker.get());
}

fn opening_is_cancellable(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let tracker = tracker();
    let f = Flyout::new();
    let count = tracker.clone();
    let _ = f.opening(move |e| {
        count.set(count.get() + 1);
        e.set_cancel(true);
    });
    f.show_at(&window);

    assert_eq!(1, tracker.get());
    assert!(!f.is_open());
}

fn closing_raises_single_closing_event(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let tracker = tracker();
    let f = Flyout::new();
    let count = tracker.clone();
    let _ = f.closing(move |_| count.set(count.get() + 1));
    f.show_at(&window);
    f.hide();

    assert_eq!(1, tracker.get());
}

fn closing_raises_single_closed_event(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let tracker = tracker();
    let f = Flyout::new();
    let count = tracker.clone();
    let _ = f.closed(move || count.set(count.get() + 1));
    f.show_at(&window);
    f.hide();

    assert_eq!(1, tracker.get());
}

fn cancel_closing_keeps_flyout_open(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let tracker = tracker();
    let f = Flyout::new();
    let count = tracker.clone();
    let _ = f.closing(move |e| {
        count.set(count.get() + 1);
        e.set_cancel(true);
    });
    f.show_at(&window);
    f.hide();

    assert!(f.is_open());
    assert_eq!(1, tracker.get());
}

fn small_top_left_button() -> Ref<Button> {
    let button = Button::new();
    button.set_height(10.0);
    button.set_width(10.0);
    button.set_horizontal_alignment(HorizontalAlignment::Left);
    button.set_vertical_alignment(VerticalAlignment::Top);
    button
}

fn cancel_light_dismiss_closing_keeps_flyout_open(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.set_width(100.0);
    window.set_height(100.0);

    let button = small_top_left_button();
    window.set_content(Some(Control::boxed(button)));

    window.show();

    let tracker = tracker();
    let f = Flyout::new();
    f.set_content(Some(Control::boxed(sized_border())));
    let count = tracker.clone();
    let _ = f.closing(move |e| {
        count.set(count.get() + 1);
        e.set_cancel(true);
    });
    f.show_at(&window);

    let e = create_pointer_pressed_event_args(&window, Point::new(90.0, 90.0));
    let overlay = LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&window);
    let overlay = overlay.expect("the window has a light dismiss overlay layer");
    overlay.raise_event(&e);

    assert_eq!(1, tracker.get());
    assert!(f.is_open());
}

fn light_dismiss_closes_flyout(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.set_width(100.0);
    window.set_height(100.0);

    let button = small_top_left_button();
    window.set_content(Some(Control::boxed(button)));

    window.show();

    let f = Flyout::new();
    f.set_content(Some(Control::boxed(sized_border())));
    f.show_at(&window);

    let e = create_pointer_pressed_event_args(&window, Point::new(90.0, 90.0));
    let overlay = LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&window);
    let overlay = overlay.expect("the window has a light dismiss overlay layer");
    overlay.raise_event(&e);

    assert!(!f.is_open());
}

/// The body of the two light dismiss tests about passing the dismiss event
/// through: returns whether the flyout is open and whether the button under
/// the pointer was clicked.
fn light_dismiss_event_to_button(use_overlay_popups: bool, pass_through: bool) -> (bool, bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.set_width(100.0);
    window.set_height(100.0);

    let button_clicked = Rc::new(Cell::new(false));
    let button = Button::new();
    button.set_click_mode(ClickMode::Press);
    let clicked = button_clicked.clone();
    button.click(move |_, _| clicked.set(true));
    window.set_content(Some(Control::boxed(button.clone())));

    window.show();

    let f = Flyout::new();
    f.set_overlay_dismiss_event_pass_through(pass_through); // Focus of test
    f.set_content(Some(Control::boxed(sized_border())));
    f.show_at(&window);

    let hit_tester = Rc::new(TestHitTester {
        point: Point::new(90.0, 90.0),
        root: RefCell::new(window.visual_root()),
        result: RefCell::new(Some(button.upcast())),
    });
    window.set_hit_tester_override(Some(hit_tester));

    let e = create_pointer_pressed_event_args(&window, Point::new(90.0, 90.0));
    let overlay = LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&window);
    let overlay = overlay.expect("the window has a light dismiss overlay layer");
    overlay.raise_event(&e);

    (f.is_open(), button_clicked.get())
}

fn light_dismiss_no_event_pass_through_to_button(use_overlay_popups: bool) {
    let (is_open, button_clicked) = light_dismiss_event_to_button(use_overlay_popups, false);

    assert!(!is_open);
    assert!(!button_clicked); // Button is NOT clicked
}

fn light_dismiss_event_pass_through_to_button(use_overlay_popups: bool) {
    let (is_open, button_clicked) = light_dismiss_event_to_button(use_overlay_popups, true);

    assert!(!is_open);
    assert!(button_clicked); // Button is clicked
}

fn flyout_has_uncancellable_close_before_showing_on_a_different_target(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    let target1 = Button::new();
    let target2 = Button::new();

    let stack_panel = StackPanel::new();
    stack_panel.children().add(target1.clone());
    stack_panel.children().add(target2.clone());
    window.set_content(Some(Control::boxed(stack_panel)));
    window.show();

    let closing_fired = Rc::new(Cell::new(false));
    let closed_fired = Rc::new(Cell::new(false));
    let f = Flyout::new();
    let fired = closing_fired.clone();
    let _ = f.closing(move |_| {
        fired.set(true); // This shouldn't happen
    });
    let fired = closed_fired.clone();
    let _ = f.closed(move || fired.set(true));

    f.show_at(&target1);

    f.show_at(&target2);

    assert!(!closing_fired.get());
    assert!(closed_fired.get());
}

/// A button whose flyout shows a panel with a focusable control (a text
/// box in the reference), and that control.
fn button_with_focusable_flyout_content(show_mode: FlyoutShowMode) -> (Ref<Button>, Ref<Border>) {
    let flyout_text_box = focusable();
    let panel = Panel::new();
    panel.children().add(flyout_text_box.clone());
    let flyout = Flyout::new();
    flyout.set_show_mode(show_mode);
    flyout.set_content(Some(Control::boxed(panel)));

    let button = Button::new();
    button.set_flyout(flyout);

    (button, flyout_text_box)
}

fn show_mode_standard_attemps_focus_flyout_content(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);

    let (button, flyout_text_box) = button_with_focusable_flyout_content(FlyoutShowMode::Standard);

    window.set_content(Some(Control::boxed(button.clone())));
    window.show();

    button.focus();
    assert_eq!(Some(button.clone().upcast::<InputElement>()), window.focus_manager().get_focused_element());
    button.flyout().unwrap().show_at(&button);
    assert!(!button.is_focused());
    assert_eq!(Some(flyout_text_box.upcast::<InputElement>()), window.focus_manager().get_focused_element());
}

fn show_mode_transient_does_not_move_focus_from_target(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);

    // The button of the reference has a text content, which is left out:
    // the services of these tests have no font manager.
    let (button, _flyout_text_box) = button_with_focusable_flyout_content(FlyoutShowMode::Transient);

    window.set_content(Some(Control::boxed(button.clone())));
    window.show();

    button.focus();
    assert_eq!(Some(button.clone().upcast::<InputElement>()), window.focus_manager().get_focused_element());
    button.flyout().unwrap().show_at(&button);
    assert_eq!(Some(button.upcast::<InputElement>()), window.focus_manager().get_focused_element());
}

fn context_requested_opens_context_flyout(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let flyout = Flyout::new();
    let target = Panel::new();
    target.set_context_flyout(&flyout);

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    let opened_count = tracker();

    let count = opened_count.clone();
    let _ = flyout.opened(move || count.set(count.get() + 1));

    target.raise_event(&ContextRequestedEventArgs::new());

    assert!(flyout.is_open());
    assert_eq!(1, opened_count.get());
}

/// A platform feedback that records what it is asked to perform.
#[derive(Default)]
struct RecordingFeedback {
    performed: RefCell<Vec<(FeedbackAction, FeedbackType)>>,
}

impl IPlatformFeedback for RecordingFeedback {
    fn perform(&self, feedback: FeedbackAction, type_: FeedbackType) -> bool {
        self.performed.borrow_mut().push((feedback, type_));
        true
    }
}

// Not in the reference suite: a context flyout opened by holding a pointer
// down performs the hold feedback of the platform; one opened by another
// context request performs none.
fn holding_opens_context_flyout_and_performs_hold_feedback(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let feedback = Rc::new(RecordingFeedback::default());
    let window_impl = create_window_mock(use_overlay_popups);
    window_impl.setup_feature::<dyn IPlatformFeedback>(feedback.clone());

    let flyout = Flyout::new();
    let target = sized_border();
    target.set_context_flyout(&flyout);
    InputElement::set_is_hold_with_mouse_enabled(&target, true);
    PlatformFeedback::set_feedback_type(&target, FeedbackType::Haptic);

    let window = Window::with_impl(window_impl);
    window.set_content(Some(Control::boxed(target.clone())));
    window.show();

    target.raise_event(&ContextRequestedEventArgs::new());

    assert!(flyout.is_open());
    assert!(feedback.performed.borrow().is_empty());

    flyout.hide();
    assert!(!flyout.is_open());

    let mouse = MouseTestHelper::new();
    mouse.down(&target);

    // The hold timer of the gesture.
    let timers = Dispatcher::timers_for_unit_tests();
    assert_eq!(1, timers.len());
    Dispatcher::force_fire_timer_for_unit_tests(&timers[0]);

    assert!(flyout.is_open());
    assert_eq!(vec![(FeedbackAction::hold(), FeedbackType::Haptic)], *feedback.performed.borrow());

    mouse.up(&target);
}

fn key_up_raised_on_target_opens_context_flyout(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let flyout = Flyout::new();
    let target = Panel::new();
    target.set_context_flyout(&flyout);
    let context_requested_count = tracker();
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

    assert!(flyout.is_open());
    assert_eq!(1, context_requested_count.get());
}

fn key_up_raised_on_target_closes_opened_context_flyout(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let flyout = Flyout::new();
    let target = Panel::new();
    target.set_context_flyout(&flyout);

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    target.raise_event(&ContextRequestedEventArgs::new());

    assert!(flyout.is_open());

    target.raise_event(&create_key_up_event_args(Key::Apps, &window));

    assert!(!flyout.is_open());
}

fn key_up_raised_on_flyout_closes_opened_context_flyout(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let flyout_content = Button::new();
    let flyout = Flyout::new();
    flyout.set_content(Some(Control::boxed(flyout_content.clone())));
    let target = Panel::new();
    target.set_context_flyout(&flyout);

    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    target.raise_event(&ContextRequestedEventArgs::new());

    assert!(flyout.is_open());

    flyout_content.raise_event(&create_key_up_event_args(Key::Apps, &window));

    assert!(!flyout.is_open());
}

/// A content control stands in for the user control of the reference test.
fn should_reset_popup_parent_on_target_detached(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let user_control = ContentControl::new();
    let window = prepared_window(Some(user_control.clone().upcast()));
    window.show();

    let flyout = Flyout::new();
    flyout.show_at(&user_control);

    let popup = flyout.popup();
    assert!(popup.parent().is_some());

    window.set_content(None);
    assert!(popup.parent().is_none());
}

/// A content control stands in for the user control of the reference test.
fn should_reset_popup_parent_on_target_attach_following_detach(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let user_control = ContentControl::new();
    let window = prepared_window(Some(user_control.clone().upcast()));
    window.show();

    let flyout = Flyout::new();
    flyout.show_at(&user_control);

    let popup = flyout.popup();
    assert!(popup.parent().is_some());

    flyout.hide();

    flyout.show_at(&user_control);
    assert!(popup.parent().is_some());
}

/// The reference builds the window and its style from markup; they are
/// built in code here.
fn context_flyout_can_be_set_in_styles(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    // The text blocks are laid out with the text services (the reference
    // services of these tests carry a font manager).
    let _text = ferroui_base::media::text_formatting::testing::TextTestScope::new();
    let menu_item = MenuItem::new();
    menu_item.items().add(boxed_str("Foo"));
    let menu_flyout = MenuFlyout::new();
    menu_flyout.items().add(Some(Control::boxed(menu_item)));
    let context_flyout: Option<Ref<FlyoutBase>> = Some(menu_flyout.upcast());

    let window = Window::new();
    window.styles().add(Style::with_setters(
        Selectors::of_type::<TextBlock>(),
        [Setter::new(Control::context_flyout_property(), context_flyout)],
    ));

    let target1 = TextBlock::new();
    target1.set_name(Some("target1".to_string()));
    let target2 = TextBlock::new();
    target2.set_name(Some("target2".to_string()));
    let panel = StackPanel::new();
    panel.children().add(target1.clone());
    panel.children().add(target2.clone());
    window.set_content(Some(Control::boxed(panel)));

    let mouse = MouseTestHelper::new();

    assert!(target1.context_flyout().is_some());
    assert!(target2.context_flyout().is_some());
    assert_eq!(target1.context_flyout(), target2.context_flyout());

    window.show();

    let menu = target1.context_flyout().unwrap();
    mouse.click_with(&target1, &target1, MouseButton::Right, None, KeyModifiers::NONE);
    assert!(menu.is_open());
    mouse.click_with(&target2, &target2, MouseButton::Right, None, KeyModifiers::NONE);
    assert!(menu.is_open());
}

/// The reference builds the window and its style from markup; the style is
/// built in code here.
fn setting_flyout_presenter_classes_sets_classes_on_flyout_presenter(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = Window::new();
    let red: Option<Rc<dyn IBrush>> = Some(Brushes::red());
    window.styles().add(Style::with_setters(
        Selectors::of_type::<FlyoutPresenter>().class("TestClass"),
        [Setter::new(TemplatedControl::background_property(), red)],
    ));

    let flyout_panel = Panel::new();
    let flyout = Flyout::new();
    flyout.set_content(Some(Control::boxed(flyout_panel.clone())));
    // The button of the reference has a text content, which is left out:
    // the services of these tests have no font manager.
    let button = Button::new();
    button.set_flyout(&flyout);
    window.set_content(Some(Control::boxed(button.clone())));
    window.show();

    flyout.flyout_presenter_classes().add("TestClass");

    button.flyout().unwrap().show_at(&button);

    let presenter = flyout_panel.get_visual_ancestors().find_map(|visual| visual.cast::<FlyoutPresenter>());
    let presenter = presenter.expect("the content of the flyout is inside a flyout presenter");
    let background = presenter.background();
    let color = background.as_ref().and_then(|brush| brush.as_solid_color_brush()).map(|brush| brush.color());
    assert_eq!(Some(Colors::RED), color);
}

fn is_open_set_false_closes_flyout(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let flyout = Flyout::new();
    let closed_fired = Rc::new(Cell::new(false));
    let fired = closed_fired.clone();
    let _ = flyout.closed(move || fired.set(true));

    flyout.show_at(&window);
    assert!(flyout.is_open());

    flyout.set_is_open(false);

    assert!(!flyout.is_open());
    assert!(!flyout.popup().is_open());
    assert!(closed_fired.get());
}

fn is_open_set_true_reopens_at_last_target(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let flyout = Flyout::new();
    flyout.show_at(&window);
    assert!(flyout.is_open());

    flyout.hide();
    assert!(!flyout.is_open());

    flyout.set_is_open(true);

    assert!(flyout.is_open());
    assert!(flyout.popup().is_open());
    assert_eq!(Some(window.upcast::<Control>()), flyout.popup().placement_target());
}

fn is_open_set_true_without_previous_target_reverts_to_false(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let flyout = Flyout::new();

    flyout.set_is_open(true);

    assert!(!flyout.is_open());
    assert!(!flyout.popup().is_open());
}

fn is_open_set_true_opens_at_button_flyout_owner(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let button = Button::new();
    let window = prepared_window(Some(button.clone().upcast()));
    window.show();

    let flyout = Flyout::new();
    button.set_flyout(&flyout);

    flyout.set_is_open(true);

    assert!(flyout.is_open());
    assert!(flyout.popup().is_open());
    assert_eq!(Some(button.upcast::<Control>()), flyout.popup().placement_target());
}

fn is_open_button_flyout_removed_clears_target(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let button = Button::new();
    let window = prepared_window(Some(button.clone().upcast()));
    window.show();

    let flyout = Flyout::new();
    button.set_flyout(&flyout);

    button.set_flyout(None);

    flyout.set_is_open(true);

    assert!(!flyout.is_open());
}

fn is_open_two_way_binding_syncs_with_source(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let view_model = FlyoutViewModel::new();
    let flyout = Flyout::new();
    let binding = ReflectionBinding::new("IsOpen").with_mode(BindingMode::TwoWay);
    binding.set_source(Some(view_model.clone() as BoxedValue));
    flyout.bind_binding(FlyoutBase::is_open_property().as_property(), &binding);

    assert!(!view_model.is_open());

    flyout.show_at(&window);
    assert!(view_model.is_open());

    flyout.hide();
    assert!(!view_model.is_open());
}

fn is_open_set_false_cancelled_closing_reverts_to_true(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let flyout = Flyout::new();
    let _ = flyout.closing(|e| e.set_cancel(true));

    flyout.show_at(&window);
    assert!(flyout.is_open());

    flyout.set_is_open(false);

    assert!(flyout.is_open());
    assert!(flyout.popup().is_open());
}

fn is_open_set_true_after_target_detached_reverts_to_false(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let target = Button::new();
    let window = prepared_window(Some(target.clone().upcast()));
    window.show();

    let flyout = Flyout::new();
    flyout.show_at(&target);
    assert!(flyout.is_open());

    // Detach the target from the visual tree
    window.set_content(None);
    assert!(!flyout.is_open());

    flyout.set_is_open(true);

    assert!(!flyout.is_open());
}

fn is_open_set_true_cancelled_opening_reverts_to_false(use_overlay_popups: bool) {
    let _app = create_services_with_focus(use_overlay_popups);
    let window = prepared_window(None);
    window.show();

    let flyout = Flyout::new();
    flyout.show_at(&window);
    flyout.hide();

    let _ = flyout.opening(|e| e.set_cancel(true));

    flyout.set_is_open(true);

    assert!(!flyout.is_open());
}

macro_rules! flyout_tests {
    ($($name:ident,)*) => {
        /// The reference `FlyoutTests`: the platform creates native popups.
        mod flyout_tests {
            $(
                #[test]
                fn $name() {
                    super::$name(false);
                }
            )*
        }

        /// The reference `OverlayPopupFlyoutTests`: the platform creates no
        /// popups, so flyouts are shown in the overlay layer.
        mod overlay_popup_flyout_tests {
            $(
                #[test]
                fn $name() {
                    super::$name(true);
                }
            )*
        }
    };
}

flyout_tests! {
    holding_opens_context_flyout_and_performs_hold_feedback,
    opening_raises_single_opening_event,
    opening_raises_single_opened_event,
    opening_is_cancellable,
    closing_raises_single_closing_event,
    closing_raises_single_closed_event,
    cancel_closing_keeps_flyout_open,
    cancel_light_dismiss_closing_keeps_flyout_open,
    light_dismiss_closes_flyout,
    light_dismiss_no_event_pass_through_to_button,
    light_dismiss_event_pass_through_to_button,
    flyout_has_uncancellable_close_before_showing_on_a_different_target,
    show_mode_standard_attemps_focus_flyout_content,
    show_mode_transient_does_not_move_focus_from_target,
    context_requested_opens_context_flyout,
    key_up_raised_on_target_opens_context_flyout,
    key_up_raised_on_target_closes_opened_context_flyout,
    key_up_raised_on_flyout_closes_opened_context_flyout,
    should_reset_popup_parent_on_target_detached,
    should_reset_popup_parent_on_target_attach_following_detach,
    context_flyout_can_be_set_in_styles,
    setting_flyout_presenter_classes_sets_classes_on_flyout_presenter,
    is_open_set_false_closes_flyout,
    is_open_set_true_reopens_at_last_target,
    is_open_set_true_without_previous_target_reverts_to_false,
    is_open_set_true_opens_at_button_flyout_owner,
    is_open_button_flyout_removed_clears_target,
    is_open_two_way_binding_syncs_with_source,
    is_open_set_false_cancelled_closing_reverts_to_true,
    is_open_set_true_after_target_detached_reverts_to_false,
    is_open_set_true_cancelled_opening_reverts_to_false,
}
