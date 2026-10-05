//! Port of the reference `WindowTests`.

use crate::platform::{
    IScreenImpl, ITopLevelImpl, IWindowBaseImpl, IWindowImpl, PlatformRequestedDrawnDecoration, Screen,
};
use crate::primitives::TemplatedControlImpl;
use crate::testing::{
    mock_screen, MockCall, MockImplKind, MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication,
};
use crate::top_level_host::TopLevelHost;
use crate::{
    Border, Canvas, ContentControlImpl, Control, ControlImpl, SizeToContent, TopLevelImpl, Window, WindowBase,
    WindowBaseImpl, WindowCloseReason, WindowImpl, WindowResizeReason, WindowStartupLocation, WindowState,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{FlowDirection, MediaContext};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::threading::Dispatcher;
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;

/// A screens implementation with fixed answers (the counterpart of a loose
/// mock with a few members set up).
#[derive(Default)]
struct TestScreenImpl {
    all_screens: Vec<Rc<Screen>>,
    screen_from_point: Option<Rc<Screen>>,
    screen_from_window: Option<Rc<Screen>>,
    changed: RefCell<Option<Rc<dyn Fn()>>>,
}

impl IScreenImpl for TestScreenImpl {
    fn screen_count(&self) -> i32 {
        0
    }

    fn all_screens(&self) -> Vec<Rc<Screen>> {
        self.all_screens.clone()
    }

    fn changed(&self) -> Option<Rc<dyn Fn()>> {
        self.changed.borrow().clone()
    }

    fn set_changed(&self, value: Option<Rc<dyn Fn()>>) {
        *self.changed.borrow_mut() = value;
    }

    fn screen_from_window(&self, _window: &dyn IWindowBaseImpl) -> Option<Rc<Screen>> {
        self.screen_from_window.clone()
    }

    fn screen_from_top_level(&self, _top_level: &dyn ITopLevelImpl) -> Option<Rc<Screen>> {
        None
    }

    fn screen_from_point(&self, _point: PixelPoint) -> Option<Rc<Screen>> {
        self.screen_from_point.clone()
    }

    fn screen_from_rect(&self, _rect: PixelRect) -> Option<Rc<Screen>> {
        None
    }

    fn request_screen_details(&self) -> Pin<Box<dyn Future<Output = bool>>> {
        Box::pin(std::future::ready(false))
    }
}

fn pixel_rect(width: i32, height: i32) -> PixelRect {
    PixelRect::from_size(PixelSize::new(width, height))
}

/// Screens whose `screen_from_point` always answers the first screen.
fn setup_screens(window_impl: &MockWindowImpl, screens: Vec<Rc<Screen>>) {
    let screens: Rc<dyn IScreenImpl> = Rc::new(TestScreenImpl {
        screen_from_point: screens.first().cloned(),
        all_screens: screens,
        ..Default::default()
    });
    window_impl.setup_feature::<dyn IScreenImpl>(screens);
}

/// A window implementation without behaviour whose screens only know the
/// screen of a window.
fn create_impl() -> Rc<MockWindowImpl> {
    let screen1 = mock_screen(1.75, pixel_rect(1920, 1080), pixel_rect(1920, 966), true);
    let screens: Rc<dyn IScreenImpl> =
        Rc::new(TestScreenImpl { screen_from_window: Some(screen1), ..Default::default() });

    let window_impl = MockWindowImpl::bare(MockImplKind::Window);
    window_impl.render_scaling.set(1.0);
    window_impl.desktop_scaling.set(0.0);
    window_impl.window_state_getter_is_usable.set(false);
    window_impl.setup_feature::<dyn IScreenImpl>(screens);

    window_impl
}

fn windowing_platform(window_impl: &Rc<MockWindowImpl>) -> Rc<MockWindowingPlatform> {
    let window_impl = window_impl.clone();
    MockWindowingPlatform::with_window_impl(move || window_impl.clone())
}

fn window_with_impl(window_impl: &Rc<MockWindowImpl>) -> Ref<Window> {
    Window::with_impl(window_impl.clone())
}

fn closed(platform_impl: &MockWindowImpl) {
    ITopLevelImpl::closed(platform_impl).expect("the closed callback is set")();
}

fn is_rendering(target: &Window) -> bool {
    MediaContext::instance().is_top_level_active(target.media_context_key())
}

/// The calls recorded by the mock implementation of a window.
fn calls(target: &Window) -> Vec<MockCall> {
    let platform_impl = target.platform_impl().expect("the window is open");
    platform_impl.as_any().downcast_ref::<MockWindowImpl>().expect("the implementation is the mock").calls()
}

fn resized(target: &Window, size: Size, reason: WindowResizeReason) {
    let platform_impl = target.platform_impl().expect("the window is open");
    platform_impl.resized().expect("the resized callback is set")(size, reason);
}

/// The message of the panic raised by `action`.
fn panic_message(action: impl FnOnce()) -> String {
    match catch_unwind(AssertUnwindSafe(action)) {
        Ok(()) => panic!("the action was expected to panic"),
        Err(payload) => payload
            .downcast_ref::<&str>()
            .map(|message| message.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .expect("the panic carries a message"),
    }
}

#[repr(C)]
struct ChildControl {
    base: Control,
    measure_sizes: RefCell<Vec<Size>>,
}

ferro_class!(ChildControl: Control);
ferro_impl_classes!(
    ChildControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl LayoutableImpl for ChildControl {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measure_sizes.borrow_mut().push(available_size);
        Self::parent_measure_override(this, available_size)
    }
}

impl ChildControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct(), measure_sizes: RefCell::new(Vec::new()) })
    }

    fn measure_sizes(&self) -> Vec<Size> {
        self.measure_sizes.borrow().clone()
    }
}

#[repr(C)]
struct TopmostWindow {
    base: Window,
}

ferro_class!(TopmostWindow: Window);
ferro_impl_classes!(
    TopmostWindow: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);

impl FerroObjectImpl for TopmostWindow {
    fn constructed(this: &Self) {
        thread_local! {
            static DONE: Cell<bool> = const { Cell::new(false) };
        }
        if !DONE.replace(true) {
            WindowBase::topmost_property().override_default_value::<TopmostWindow>(true);
        }
        Self::parent_constructed(this);
    }
}

impl TopmostWindow {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Window::construct(crate::platform::PlatformManager::create_window()) })
    }
}

#[test]
fn setting_title_should_set_impl_title() {
    let window_impl = MockWindowImpl::bare(MockImplKind::Window);
    window_impl.render_scaling.set(1.0);
    let windowing_platform = windowing_platform(&window_impl);

    let _app = UnitTestApplication::start(TestServices::new().with_windowing_platform(windowing_platform));
    let target = Window::new();

    target.set_title(Some("Hello World".to_string()));

    assert_eq!(1, window_impl.count_of(&MockCall::SetTitle(Some("Hello World".to_string()))));
}

#[test]
fn is_visible_should_initially_be_false() {
    let _app = UnitTestApplication::start(TestServices::mock_windowing_platform());
    let window = Window::new();

    assert!(!window.is_visible());
}

#[test]
fn is_visible_should_be_true_after_show() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();

    window.show();

    assert!(window.is_visible());
}

#[test]
fn is_visible_should_be_true_after_show_dialog() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    parent.show();
    let window = Window::new();

    let _task = window.show_dialog(&parent);

    assert!(window.is_visible());
}

#[test]
fn is_visible_should_be_false_after_hide() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();

    window.show();
    window.hide();

    assert!(!window.is_visible());
}

#[test]
fn is_visible_should_be_false_after_close() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();

    window.show();
    window.close();

    assert!(!window.is_visible());
}

#[test]
fn is_visible_should_be_false_after_impl_signals_close() {
    let window_impl = create_impl();
    window_impl.desktop_scaling.set(1.0);

    let services = TestServices::styled_window().with_windowing_platform(windowing_platform(&window_impl));

    let _app = UnitTestApplication::start(services);
    let window = Window::new();

    window.show();
    assert!(window.is_visible());

    closed(&window_impl);

    assert!(!window.is_visible());
}

#[test]
fn closing_should_only_be_invoked_once() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    let count = Rc::new(Cell::new(0));

    let _ = window.closing({
        let count = count.clone();
        move |_| count.set(count.get() + 1)
    });

    window.show();
    window.close();

    assert_eq!(1, count.get());
}

/// Counts the events of a window and its child in the order they are
/// raised.
#[derive(Default)]
struct CloseOrder {
    count: Cell<i32>,
    window_closing: Cell<i32>,
    child_closing: Cell<i32>,
    window_closed: Cell<i32>,
    child_closed: Cell<i32>,
}

impl CloseOrder {
    fn next(&self) -> i32 {
        self.count.set(self.count.get() + 1);
        self.count.get()
    }
}

fn child_windows_should_be_closed_before_parent(programmatic_close: bool) {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    let child = Window::new();

    let order = Rc::new(CloseOrder::default());

    let _ = window.closing({
        let order = order.clone();
        move |e| {
            assert_eq!(WindowCloseReason::WindowClosing, e.close_reason());
            assert_eq!(programmatic_close, e.is_programmatic());
            order.window_closing.set(order.next());
        }
    });

    let _ = child.closing({
        let order = order.clone();
        move |e| {
            assert_eq!(WindowCloseReason::OwnerWindowClosing, e.close_reason());
            assert_eq!(programmatic_close, e.is_programmatic());
            order.child_closing.set(order.next());
        }
    });

    let _ = window.closed({
        let order = order.clone();
        move || order.window_closed.set(order.next())
    });

    let _ = child.closed({
        let order = order.clone();
        move || order.child_closed.set(order.next())
    });

    window.show();
    child.show_with_owner(&window);

    if programmatic_close {
        window.close();
    } else {
        let platform_impl = window.platform_impl().unwrap();
        let cancel = platform_impl.closing().unwrap()(WindowCloseReason::WindowClosing);

        assert!(!cancel);
    }

    assert_eq!(2, order.window_closing.get());
    assert_eq!(1, order.child_closing.get());
    assert_eq!(4, order.window_closed.get());
    assert_eq!(3, order.child_closed.get());
}

#[test]
fn child_windows_should_be_closed_before_parent_programmatic() {
    child_windows_should_be_closed_before_parent(true);
}

#[test]
fn child_windows_should_be_closed_before_parent_from_platform() {
    child_windows_should_be_closed_before_parent(false);
}

fn child_windows_must_not_close_before_parent_has_chance_to_cancel_os_close_button(programmatic_close: bool) {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    let child = Window::new();

    let order = Rc::new(CloseOrder::default());

    let _ = window.closing({
        let order = order.clone();
        move |e| {
            order.window_closing.set(order.next());
            e.set_cancel(true);
        }
    });

    let _ = child.closing({
        let order = order.clone();
        move |_| order.child_closing.set(order.next())
    });

    let _ = window.closed({
        let order = order.clone();
        move || order.window_closed.set(order.next())
    });

    let _ = child.closed({
        let order = order.clone();
        move || order.child_closed.set(order.next())
    });

    window.show();
    child.show_with_owner(&window);

    if programmatic_close {
        window.close();
    } else {
        let platform_impl = window.platform_impl().unwrap();
        let cancel = platform_impl.closing().unwrap()(WindowCloseReason::WindowClosing);

        assert!(cancel);
    }

    assert_eq!(2, order.window_closing.get());
    assert_eq!(1, order.child_closing.get());
    assert_eq!(0, order.window_closed.get());
    assert_eq!(0, order.child_closed.get());
}

#[test]
fn child_windows_must_not_close_before_parent_has_chance_to_cancel_os_close_button_programmatic() {
    child_windows_must_not_close_before_parent_has_chance_to_cancel_os_close_button(true);
}

#[test]
fn child_windows_must_not_close_before_parent_has_chance_to_cancel_os_close_button_from_platform() {
    child_windows_must_not_close_before_parent_has_chance_to_cancel_os_close_button(false);
}

#[test]
fn showing_should_start_renderer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = window_with_impl(&create_impl());

    target.show();
    assert!(is_rendering(&target));
}

#[test]
fn show_dialog_should_start_renderer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    let target = window_with_impl(&create_impl());

    parent.show();
    let _task = target.show_dialog(&parent);

    assert!(is_rendering(&target));
}

#[test]
fn show_dialog_should_raise_opened() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    let target = Window::new();
    let raised = Rc::new(Cell::new(false));

    parent.show();
    let _ = target.opened({
        let raised = raised.clone();
        move || raised.set(true)
    });

    let _task = target.show_dialog(&parent);

    assert!(raised.get());
}

#[test]
fn hiding_should_stop_renderer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = window_with_impl(&create_impl());

    target.show();
    target.hide();
    assert!(!is_rendering(&target));
}

#[test]
fn show_dialog_with_value_type_returns_default_when_closed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    let window_impl = create_impl();
    window_impl.desktop_scaling.set(1.0);
    window_impl.render_scaling.set(1.0);

    parent.show();
    let target = window_with_impl(&window_impl);
    let task = target.show_dialog_typed::<bool>(&parent);

    closed(&window_impl);

    Dispatcher::ui_thread().run_jobs(None);
    assert!(task.is_completed());
    let result = task.result().unwrap();
    assert!(!result);
}

#[test]
fn show_dialog_returns_the_dialog_result_when_closed_with_a_result() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    parent.show();

    let untyped = Window::new();
    let untyped_task = untyped.show_dialog(&parent);
    let typed = Window::new();
    let typed_task = typed.show_dialog_typed::<i32>(&parent);

    Dispatcher::ui_thread().run_jobs(None);
    assert!(!untyped_task.is_completed());
    assert!(!typed_task.is_completed());

    let result: BoxedValue = Rc::new("done".to_string());
    untyped.close_with_result(Some(result));
    let result: BoxedValue = Rc::new(42);
    typed.close_with_result(Some(result));

    Dispatcher::ui_thread().run_jobs(None);
    assert!(untyped_task.is_completed());
    let untyped_result = untyped_task.result().unwrap().expect("the dialog was closed with a result");
    assert_eq!(Some(&"done".to_string()), untyped_result.downcast_ref::<String>());
    assert_eq!(42, typed_task.result().unwrap());
}

#[test]
fn calling_show_on_closed_window_should_throw() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Window::new();

    target.show();
    target.close();

    let opened_raised = Rc::new(Cell::new(false));
    let _ = target.opened({
        let opened_raised = opened_raised.clone();
        move || opened_raised.set(true)
    });

    let message = panic_message(|| target.show());
    assert_eq!("Cannot re-show a closed window.", message);
    assert!(!opened_raised.get());
}

#[test]
fn calling_show_dialog_on_closed_window_should_throw() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    let window_impl = create_impl();
    window_impl.desktop_scaling.set(1.0);
    window_impl.render_scaling.set(1.0);

    parent.show();

    let target = window_with_impl(&window_impl);
    let task = target.show_dialog_typed::<bool>(&parent);

    closed(&window_impl);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(task.is_completed());

    let opened_raised = Rc::new(Cell::new(false));
    let _ = target.opened({
        let opened_raised = opened_raised.clone();
        move || opened_raised.set(true)
    });

    let message = panic_message(|| {
        let _ = target.show_dialog_typed::<bool>(&parent);
    });
    assert_eq!("Cannot re-show a closed window.", message);
    assert!(!opened_raised.get());
}

#[test]
fn calling_show_with_closed_parent_window_should_throw() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    let target = Window::new();

    parent.close();

    let message = panic_message(|| target.show_with_owner(&parent));
    assert_eq!("Cannot show a window with a closed owner.", message);
}

#[test]
fn calling_show_dialog_with_closed_parent_window_should_throw() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    let target = Window::new();

    parent.close();

    let message = panic_message(|| {
        let _ = target.show_dialog(&parent);
    });
    assert_eq!("Cannot show a window with a closed owner.", message);
}

#[test]
fn calling_show_with_invisible_parent_window_should_throw() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    let target = Window::new();

    let message = panic_message(|| target.show_with_owner(&parent));
    assert_eq!("Cannot show window with non-visible owner.", message);
}

#[test]
fn calling_show_dialog_with_invisible_parent_window_should_throw() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    let target = Window::new();

    let message = panic_message(|| {
        let _ = target.show_dialog(&parent);
    });
    assert_eq!("Cannot show window with non-visible owner.", message);
}

#[test]
fn calling_show_with_self_as_parent_window_should_throw() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Window::new();

    let message = panic_message(|| target.show_with_owner(&target));
    assert_eq!("A Window cannot be its own owner.", message);
}

#[test]
fn calling_show_dialog_with_self_as_parent_window_should_throw() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Window::new();

    let message = panic_message(|| {
        let _ = target.show_dialog(&target);
    });
    assert_eq!("A Window cannot be its own owner.", message);
}

#[test]
fn hiding_parent_window_should_close_children() {
    let _app = UnitTestApplication::start(TestServices::mock_windowing_platform());
    let parent = Window::new();
    let child = Window::new();

    parent.show();
    child.show_with_owner(&parent);

    parent.hide();

    assert!(!parent.is_visible());
    assert!(!child.is_visible());
}

#[test]
fn hiding_parent_window_should_close_dialog_children() {
    let _app = UnitTestApplication::start(TestServices::mock_windowing_platform());
    let parent = Window::new();
    let child = Window::new();

    parent.show();
    let _task = child.show_dialog(&parent);

    parent.hide();

    assert!(!parent.is_visible());
    assert!(!child.is_visible());
}

#[test]
fn window_should_not_be_centered_when_window_startup_location_is_center_screen_and_window_is_hidden_and_shown() {
    let screen1 = mock_screen(1.0, pixel_rect(1920, 1080), pixel_rect(1920, 1040), true);

    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.client_size.set(Size::new(800.0, 480.0));
    window_impl.desktop_scaling.set(1.0);
    window_impl.render_scaling.set(1.0);
    setup_screens(&window_impl, vec![screen1]);

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = window_with_impl(&window_impl);
    window.set_window_startup_location(WindowStartupLocation::CenterScreen);

    window.show();

    let expected = PixelPoint::new(150, 400);
    window.set_position(expected);

    window.set_is_visible(false);
    window.set_is_visible(true);

    assert_eq!(expected, window.position());
}

#[test]
fn window_should_be_centered_when_window_startup_location_is_center_screen() {
    let screen1 = mock_screen(1.0, pixel_rect(1920, 1080), pixel_rect(1920, 1040), true);
    let screen2 = mock_screen(1.0, pixel_rect(1366, 768), pixel_rect(1366, 728), false);

    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.client_size.set(Size::new(800.0, 480.0));
    window_impl.desktop_scaling.set(1.0);
    window_impl.render_scaling.set(1.0);
    setup_screens(&window_impl, vec![screen1.clone(), screen2]);

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = window_with_impl(&window_impl);
    window.set_window_startup_location(WindowStartupLocation::CenterScreen);
    window.set_position(PixelPoint::new(60, 40));

    window.show();

    let expected_position = PixelPoint::new(
        (screen1.working_area().size().width as f64 / 2.0 - window.client_size().width / 2.0) as i32,
        (screen1.working_area().size().height as f64 / 2.0 - window.client_size().height / 2.0) as i32,
    );

    assert_eq!(window.position(), expected_position);
}

#[test]
fn window_should_be_sized_to_min_size_if_initial_size_less_than_min_size() {
    let screen1 = mock_screen(1.75, pixel_rect(1920, 1080), pixel_rect(1920, 966), true);

    let window_impl = MockWindowingPlatform::create_window_mock_with_size(400.0, 300.0);
    window_impl.desktop_scaling.set(1.75);
    window_impl.render_scaling.set(1.75);
    setup_screens(&window_impl, vec![screen1]);

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = window_with_impl(&window_impl);
    window.set_window_startup_location(WindowStartupLocation::CenterScreen);
    window.set_min_width(720.0);
    window.set_min_height(480.0);

    window.show();

    assert_eq!(PixelPoint::new(330, 63), window.position());
    assert_eq!(Size::new(720.0, 480.0), window.bounds().size());
}

#[test]
fn window_should_be_centered_relative_to_owner_when_window_startup_location_is_center_owner() {
    let parent_window_impl = MockWindowingPlatform::create_window_mock();
    parent_window_impl.client_size.set(Size::new(800.0, 480.0));
    parent_window_impl.max_auto_size_hint.set(Size::new(1920.0, 1080.0));
    parent_window_impl.desktop_scaling.set(1.0);
    parent_window_impl.render_scaling.set(1.0);

    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.client_size.set(Size::new(320.0, 200.0));
    window_impl.max_auto_size_hint.set(Size::new(1920.0, 1080.0));
    window_impl.desktop_scaling.set(1.0);
    window_impl.render_scaling.set(1.0);

    // The reference test starts a second application to create the second window from another
    // windowing platform; here both windows are created over their implementations.
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent_window = window_with_impl(&parent_window_impl);
    parent_window.set_position(PixelPoint::new(60, 40));

    parent_window.show();

    let window = window_with_impl(&window_impl);
    window.set_window_startup_location(WindowStartupLocation::CenterOwner);
    window.set_position(PixelPoint::new(60, 40));

    let _task = window.show_dialog(&parent_window);

    let expected_position = PixelPoint::new(
        (parent_window.position().x as f64 + parent_window.client_size().width / 2.0
            - window.client_size().width / 2.0) as i32,
        (parent_window.position().y as f64 + parent_window.client_size().height / 2.0
            - window.client_size().height / 2.0) as i32,
    );

    assert_eq!(window.position(), expected_position);
}

#[test]
fn window_topmost_by_default_should_configure_platform_impl_when_constructed() {
    let window_impl = MockWindowingPlatform::create_window_mock();

    let window_services = TestServices::styled_window().with_windowing_platform(windowing_platform(&window_impl));

    let _app = UnitTestApplication::start(window_services);
    let window = TopmostWindow::new();

    assert!(window.topmost());
    assert_eq!(1, window_impl.count_of(&MockCall::SetTopmost(true)));
}

#[test]
fn can_maximize_should_be_false_if_can_resize_is_false() {
    let window_impl = MockWindowingPlatform::create_window_mock();

    let _app = UnitTestApplication::start(
        TestServices::styled_window().with_windowing_platform(windowing_platform(&window_impl)),
    );

    let window = Window::new();

    assert!(window.can_maximize());

    window.set_can_resize(false);

    assert!(!window.can_maximize());
}

#[test]
fn flow_direction_rtl_should_not_result_in_mirrored_host() {
    let window_impl = MockWindowingPlatform::create_window_mock();

    let _app = UnitTestApplication::start(
        TestServices::styled_window().with_windowing_platform(windowing_platform(&window_impl)),
    );

    let window = Window::new();
    window.set_flow_direction(FlowDirection::RightToLeft);

    let visual_root = window.visual_root().expect("the window has a visual root");
    assert!(visual_root.is::<TopLevelHost>());

    assert!(!window.has_mirror_transform());
    assert!(!visual_root.has_mirror_transform());
}

/// Without the drawn decorations (not ported yet) this only covers the
/// notification arriving while the content is being attached.
#[test]
fn extending_client_area_to_decorations_when_attached_to_visual_tree_works() {
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.requested_drawn_decorations.set(PlatformRequestedDrawnDecoration::TITLE_BAR);

    let _app = UnitTestApplication::start(
        TestServices::styled_window().with_windowing_platform(windowing_platform(&window_impl)),
    );

    let border = Border::new();

    let window = Window::new();
    window.set_content(Some(Control::boxed(border.clone())));

    let _ = border.attached_to_visual_tree({
        let window_impl = window_impl.clone();
        move |_| {
            window_impl.needs_managed_decorations.set(true);
            if let Some(changed) = window_impl.extend_client_area_to_decorations_changed() {
                changed(true);
            }
        }
    });

    window.show();

    assert!(window.is_extended_into_window_decorations());
}

/// The sizing tests, once for windows shown with `show` and once for
/// windows shown as dialogs. `$show` shows the window and returns what has
/// to stay alive while the test runs.
macro_rules! sizing_tests {
    ($name:ident, $show:expr) => {
        mod $name {
            use super::*;

            fn show(window: &Window) -> Option<Ref<Window>> {
                let show: fn(&Window) -> Option<Ref<Window>> = $show;
                show(window)
            }

            #[test]
            fn child_should_be_measured_with_width_and_height_if_size_to_content_is_manual() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = ChildControl::new();
                let target = Window::new();
                target.set_width(100.0);
                target.set_height(50.0);
                target.set_size_to_content(SizeToContent::MANUAL);
                target.set_content(Some(Control::boxed(child.clone())));

                // Verify that the child is initially measured with our Width/Height.
                let _owner = show(&target);

                assert_eq!(1, child.measure_sizes().len());
                assert_eq!(Size::new(100.0, 50.0), child.measure_sizes()[0]);

                // Now change the bounds: verify that we are using the new Width/Height, and not the old ClientSize.
                child.measure_sizes.borrow_mut().clear();
                child.invalidate_measure();

                target.set_width(120.0);
                target.set_height(70.0);

                Dispatcher::ui_thread().run_jobs(None);

                assert_eq!(1, child.measure_sizes().len());
                assert_eq!(Size::new(120.0, 70.0), child.measure_sizes()[0]);
            }

            #[test]
            fn child_should_be_measured_with_client_size_if_size_to_content_is_manual_and_no_width_height_specified() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let window_impl = MockWindowingPlatform::create_window_mock();
                window_impl.client_size.set(Size::new(550.0, 450.0));

                let child = ChildControl::new();
                let target = window_with_impl(&window_impl);
                target.set_size_to_content(SizeToContent::MANUAL);
                target.set_content(Some(Control::boxed(child.clone())));

                let _owner = show(&target);

                assert_eq!(1, child.measure_sizes().len());
                assert_eq!(Size::new(550.0, 450.0), child.measure_sizes()[0]);
            }

            #[test]
            fn child_should_be_measured_with_max_auto_size_hint_if_size_to_content_is_width_and_height() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let window_impl = MockWindowingPlatform::create_window_mock();
                window_impl.max_auto_size_hint.set(Size::new(1200.0, 1000.0));

                let child = ChildControl::new();
                let target = window_with_impl(&window_impl);
                target.set_width(100.0);
                target.set_height(50.0);
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_content(Some(Control::boxed(child.clone())));

                target.show();

                assert_eq!(1, child.measure_sizes().len());
                assert_eq!(Size::new(1200.0, 1000.0), child.measure_sizes()[0]);
            }

            #[test]
            fn should_not_have_offset_on_bounds_when_content_larger_than_max_window_size() {
                // Issue #3784.
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let window_impl = MockWindowingPlatform::create_window_mock();
                let max_client_size = Size::new(480.0, 480.0);

                window_impl.client_size.set(Size::new(200.0, 200.0));
                window_impl.setup_resize(move |window_impl, size, reason| {
                    let client_size = size.constrain(max_client_size);
                    window_impl.client_size.set(client_size);
                    if let Some(resized) = ITopLevelImpl::resized(window_impl) {
                        resized(client_size, reason);
                    }
                });

                let child = Canvas::new();
                child.set_width(400.0);
                child.set_height(800.0);
                let target = window_with_impl(&window_impl);
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_content(Some(Control::boxed(child)));

                let _owner = show(&target);

                assert_eq!(Size::new(400.0, 480.0), target.bounds().size());

                // Issue #3784 causes this to be (0, 160) which makes no sense as Window has no
                // parent control to be offset against.
                assert_eq!(Point::new(0.0, 0.0), target.bounds().position());
            }

            #[test]
            fn width_height_should_not_be_nan_after_show_with_size_to_content_manual() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = Canvas::new();
                child.set_width(400.0);
                child.set_height(800.0);

                let target = Window::new();
                target.set_size_to_content(SizeToContent::MANUAL);
                target.set_content(Some(Control::boxed(child)));

                let _owner = show(&target);

                // Values come from the defaults of the mock windowing platform.
                assert_eq!(800.0, target.width());
                assert_eq!(600.0, target.height());
            }

            #[test]
            fn width_height_should_not_be_nan_after_show_with_size_to_content_width_and_height() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = Canvas::new();
                child.set_width(400.0);
                child.set_height(800.0);

                let target = Window::new();
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_content(Some(Control::boxed(child)));

                let object: &FerroObject = &target;
                let _ = object.get_observable(Layoutable::width_property()).subscribe_fn(|_| {});

                let _owner = show(&target);

                assert_eq!(400.0, target.width());
                assert_eq!(800.0, target.height());
            }

            #[test]
            fn max_width_and_max_height_should_be_respected_with_size_to_content_width_and_height() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = ChildControl::new();

                let target = Window::new();
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_max_width(300.0);
                target.set_max_height(700.0);
                target.set_content(Some(Control::boxed(child.clone())));

                let _owner = show(&target);

                assert_eq!(vec![Size::new(300.0, 700.0)], child.measure_sizes());
            }

            #[test]
            fn size_to_content_should_not_be_lost_on_show() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = Canvas::new();
                child.set_width(400.0);
                child.set_height(800.0);

                let target = Window::new();
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_content(Some(Control::boxed(child)));

                let _owner = show(&target);

                assert_eq!(SizeToContent::WIDTH_AND_HEIGHT, target.size_to_content());
            }

            #[test]
            fn size_to_content_should_not_be_lost_on_scaling_change() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = Canvas::new();
                child.set_width(209.0);
                child.set_height(117.0);

                let target = Window::new();
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_content(Some(Control::boxed(child)));

                let _owner = show(&target);

                // Size before and after DPI change is a real-world example, with size after DPI
                // change coming from Win32 WM_DPICHANGED.
                let platform_impl = target.platform_impl().unwrap();
                platform_impl.scaling_changed().unwrap()(1.5);
                platform_impl.resized().unwrap()(
                    Size::new(210.66666666666666, 118.66666666666667),
                    WindowResizeReason::DpiChange,
                );

                assert_eq!(SizeToContent::WIDTH_AND_HEIGHT, target.size_to_content());
            }

            #[test]
            fn width_height_should_be_updated_when_size_to_content_is_width_and_height() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = Canvas::new();
                child.set_width(400.0);
                child.set_height(800.0);

                let target = Window::new();
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_content(Some(Control::boxed(child.clone())));

                let _owner = show(&target);

                assert_eq!(400.0, target.width());
                assert_eq!(800.0, target.height());

                child.set_width(410.0);
                target.layout_manager().execute_layout_pass();

                assert_eq!(410.0, target.width());
                assert_eq!(800.0, target.height());
                assert_eq!(SizeToContent::WIDTH_AND_HEIGHT, target.size_to_content());
            }

            #[test]
            fn setting_width_should_resize_window_impl() {
                // Issue #3796
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let target = Window::new();
                target.set_width(400.0);
                target.set_height(800.0);

                let _owner = show(&target);

                assert_eq!(400.0, target.width());
                assert_eq!(800.0, target.height());

                target.set_width(410.0);
                target.layout_manager().execute_layout_pass();

                let resize = MockCall::Resize(Size::new(410.0, 800.0), WindowResizeReason::Application);
                assert!(calls(&target).contains(&resize));
                assert_eq!(410.0, target.width());
            }

            #[test]
            fn user_resize_of_window_width_should_reset_size_to_content() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = Canvas::new();
                child.set_width(400.0);
                child.set_height(800.0);
                let target = Window::new();
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_content(Some(Control::boxed(child)));

                let _owner = show(&target);
                assert_eq!(400.0, target.width());
                assert_eq!(800.0, target.height());

                resized(&target, Size::new(410.0, 800.0), WindowResizeReason::User);

                assert_eq!(410.0, target.width());
                assert_eq!(800.0, target.height());
                assert_eq!(SizeToContent::HEIGHT, target.size_to_content());
            }

            #[test]
            fn user_resize_of_window_height_should_reset_size_to_content() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = Canvas::new();
                child.set_width(400.0);
                child.set_height(800.0);
                let target = Window::new();
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_content(Some(Control::boxed(child)));

                let _owner = show(&target);
                assert_eq!(400.0, target.width());
                assert_eq!(800.0, target.height());

                resized(&target, Size::new(400.0, 810.0), WindowResizeReason::User);

                assert_eq!(400.0, target.width());
                assert_eq!(810.0, target.height());
                assert_eq!(SizeToContent::WIDTH, target.size_to_content());
            }

            #[test]
            fn window_resize_should_not_reset_size_to_content_if_can_resize_false() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let child = Canvas::new();
                child.set_width(400.0);
                child.set_height(800.0);
                let target = Window::new();
                target.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
                target.set_can_resize(false);
                target.set_content(Some(Control::boxed(child)));

                let _owner = show(&target);
                assert_eq!(400.0, target.width());
                assert_eq!(800.0, target.height());

                resized(&target, Size::new(410.0, 810.0), WindowResizeReason::Unspecified);

                assert_eq!(400.0, target.width());
                assert_eq!(800.0, target.height());
                assert_eq!(SizeToContent::WIDTH_AND_HEIGHT, target.size_to_content());
            }

            #[test]
            fn is_visible_should_open_window() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let target = Window::new();
                let raised = Rc::new(Cell::new(false));

                let _ = target.opened({
                    let raised = raised.clone();
                    move || raised.set(true)
                });
                target.set_is_visible(true);

                assert!(raised.get());
            }

            #[test]
            fn hiding_dialog_window_should_complete_task() {
                let _app = UnitTestApplication::start(TestServices::styled_window());
                let parent = Window::new();
                parent.show();

                let target = Window::new();

                let task = target.show_dialog_typed::<bool>(&parent);

                target.set_is_visible(false);

                Dispatcher::ui_thread().run_jobs(None);
                assert!(task.is_completed_successfully());
            }

            #[test]
            fn show_works_when_min_dimension_greater_than_max() {
                let _app = UnitTestApplication::start(TestServices::styled_window());

                let target = Window::new();
                target.set_min_width(100.0);
                target.set_max_width(80.0);
                target.set_min_height(200.0);
                target.set_max_height(180.0);

                let _owner = show(&target);

                assert_eq!(100.0, target.width());
                assert_eq!(200.0, target.height());
            }
        }
    };
}

sizing_tests!(sizing_tests, |window| {
    window.show();
    None
});

sizing_tests!(dialog_sizing_tests, |window| {
    let owner = Window::new();
    owner.show();
    let _ = window.show_dialog(&owner);
    Some(owner)
});

#[test]
fn show_should_apply_default_icon_when_no_custom_icon_is_set() {
    let window_impl = MockWindowingPlatform::create_window_mock();
    let windowing_platform = windowing_platform(&window_impl);

    let _app = UnitTestApplication::start(TestServices::styled_window().with_windowing_platform(windowing_platform));
    let target = Window::new();

    // Clear any icon calls from construction.
    window_impl.clear_calls();

    target.show();

    // Showing should apply the effective icon when no custom icon was set.
    assert!(window_impl.count(|call| matches!(call, MockCall::SetIcon(_))) >= 1);
}

#[test]
fn window_state_usable_getter_setter_updates_only_after_platform_callback() {
    let window_impl = MockWindowingPlatform::create_window_mock();

    // Simulate a platform where the getter is usable and the setter is accepted
    window_impl.window_state_getter_is_usable.set(true);
    window_impl.setup_set_window_state(|window_impl, v| {
        // Platform accepts the state and fires the callback
        if let Some(window_state_changed) = window_impl.window_state_changed() {
            window_state_changed(v);
        }
    });

    let windowing_platform = windowing_platform(&window_impl);
    let _app = UnitTestApplication::start(TestServices::new().with_windowing_platform(windowing_platform));
    let target = Window::new();
    target.show();

    let raised = Rc::new(RefCell::new(Vec::new()));
    let skip = Cell::new(true);
    let object: &FerroObject = &target;
    let _ = object.get_observable(Window::window_state_property()).subscribe_fn({
        let raised = raised.clone();
        move |s: WindowState| {
            if !skip.replace(false) {
                raised.borrow_mut().push(s);
            }
        }
    });

    // Set to Maximized - platform accepts and fires callback
    target.set_window_state(WindowState::Maximized);
    assert_eq!(WindowState::Maximized, target.window_state());
    assert!(raised.borrow().contains(&WindowState::Maximized));

    // Set to FullScreen - platform accepts and fires callback
    raised.borrow_mut().clear();
    target.set_window_state(WindowState::FullScreen);
    assert_eq!(WindowState::FullScreen, target.window_state());
    assert!(raised.borrow().contains(&WindowState::FullScreen));
}

#[test]
fn window_state_usable_getter_setter_raises_synthetic_notification_when_platform_refuses() {
    let window_impl = MockWindowingPlatform::create_window_mock();

    // Simulate a platform where the getter is usable but refuses state change requests.
    // Start in Maximized state, then refuse a request to go Normal.
    let platform_state = Rc::new(Cell::new(WindowState::Normal));
    window_impl.window_state_getter_is_usable.set(true);
    window_impl.setup_set_window_state({
        let platform_state = platform_state.clone();
        move |window_impl, v| {
            // Platform accepts Maximized but refuses everything else
            if v == WindowState::Maximized {
                platform_state.set(v);
                if let Some(window_state_changed) = window_impl.window_state_changed() {
                    window_state_changed(v);
                }
            } else {
                // platform refuses, does not change state
                window_impl.window_state.set(platform_state.get());
            }
        }
    });

    let windowing_platform = windowing_platform(&window_impl);
    let _app = UnitTestApplication::start(TestServices::new().with_windowing_platform(windowing_platform));
    let target = Window::new();
    target.show();

    // First, go to Maximized (accepted by platform)
    target.set_window_state(WindowState::Maximized);
    assert_eq!(WindowState::Maximized, target.window_state());

    let raised = Rc::new(RefCell::new(Vec::new()));
    let _ = target.property_changed({
        let raised = raised.clone();
        move |e| {
            if e.property() == Window::window_state_property().as_property() {
                raised.borrow_mut().push(e.get_new_value::<WindowState>());
            }
        }
    });

    // Now try to go to FullScreen - platform refuses, stays Maximized
    target.set_window_state(WindowState::FullScreen);

    // The getter should still return Maximized because the platform refused
    assert_eq!(WindowState::Maximized, target.window_state());

    // A synthetic notification should have been raised so data bindings can recover
    assert!(!raised.borrow().is_empty());
    assert_eq!(Some(&WindowState::Maximized), raised.borrow().last());
}

#[test]
fn window_state_non_usable_getter_setter_updates_immediately() {
    let window_impl = MockWindowingPlatform::create_window_mock();

    // Legacy behavior: the window state getter is not usable
    window_impl.window_state_getter_is_usable.set(false);

    let windowing_platform = windowing_platform(&window_impl);
    let _app = UnitTestApplication::start(TestServices::new().with_windowing_platform(windowing_platform));
    let target = Window::new();
    target.show();

    let raised = Rc::new(RefCell::new(Vec::new()));
    let skip = Cell::new(true);
    let object: &FerroObject = &target;
    let _ = object.get_observable(Window::window_state_property()).subscribe_fn({
        let raised = raised.clone();
        move |s: WindowState| {
            if !skip.replace(false) {
                raised.borrow_mut().push(s);
            }
        }
    });

    // Set to Maximized - should update immediately regardless of platform behavior
    target.set_window_state(WindowState::Maximized);

    assert_eq!(WindowState::Maximized, target.window_state());
    assert!(raised.borrow().contains(&WindowState::Maximized));

    // Verify the setter was forwarded to the platform impl
    assert_eq!(1, window_impl.count_of(&MockCall::SetWindowState(WindowState::Maximized)));

    // Platform getter should never be called in legacy mode
    assert_eq!(0, window_impl.window_state_get_count.get());
}

// Additional tests of members the reference suite does not cover.

#[test]
fn dialog_is_owned_and_disables_its_owner_until_it_closes() {
    let parent_impl = MockWindowingPlatform::create_window_mock();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = window_with_impl(&parent_impl);
    parent.show();
    let dialog = Window::new();

    let task = dialog.show_dialog(&parent);

    assert!(dialog.is_dialog());
    assert_eq!(1, parent.owned_windows().len());
    assert!(parent.owned_windows()[0].ptr_eq(&dialog));
    assert!(dialog.owner().is_some_and(|owner| owner.ptr_eq(&parent)));
    assert_eq!(Some(&MockCall::SetEnabled(false)), parent_impl.calls().last());

    parent_impl.clear_calls();
    dialog.close();

    assert!(!dialog.is_dialog());
    assert!(parent.owned_windows().is_empty());
    assert!(dialog.owner().is_none());
    assert!(parent_impl.calls().contains(&MockCall::SetEnabled(true)));
    assert_eq!(1, parent_impl.count_of(&MockCall::Activate));

    Dispatcher::ui_thread().run_jobs(None);
    assert!(task.is_completed_successfully());
    assert!(task.result().unwrap().is_none());
}

#[test]
fn closed_window_is_released() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let parent = Window::new();
    parent.show();
    let dialog = Window::new();
    let task = dialog.show_dialog(&parent);

    dialog.close();
    parent.close();
    Dispatcher::ui_thread().run_jobs(None);
    drop(task);

    let weak_dialog = dialog.downgrade();
    let weak_parent = parent.downgrade();
    drop(dialog);
    drop(parent);

    assert!(weak_dialog.upgrade().is_none());
    assert!(weak_parent.upgrade().is_none());
}

#[test]
fn shown_window_is_kept_alive_by_its_platform_impl_until_it_closes() {
    let window_impl = MockWindowingPlatform::create_window_mock();
    let _app = UnitTestApplication::start(TestServices::styled_window().with_windowing_platform(windowing_platform(&window_impl)));

    let weak = {
        let window = Window::new();
        window.show();
        window.downgrade()
    };
    Dispatcher::ui_thread().run_jobs(None);

    // (a) Nothing but the platform implementation references the window.
    assert!(weak.upgrade().is_some());
    assert!(weak.upgrade().unwrap().is_visible());

    ITopLevelImpl::closed(&*window_impl).expect("the closed callback is set while the window is open")();
    Dispatcher::ui_thread().run_jobs(None);

    // (b) The closed window is released.
    assert!(weak.upgrade().is_none());
}
