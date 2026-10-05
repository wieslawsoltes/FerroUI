//! Port of the reference `WindowBaseTests`.

use crate::platform::{ITopLevelImpl, IWindowBaseImpl};
use crate::primitives::TemplatedControlImpl;
use crate::testing::{MockCall, MockImplKind, MockWindowImpl, NullRenderer, TestServices, UnitTestApplication};
use crate::{ContentControlImpl, ControlImpl, TopLevelImpl, WindowBase, WindowBaseImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::MediaContext;
use ferroui_base::*;
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
struct TestWindowBase {
    base: WindowBase,
}

ferro_class!(TestWindowBase: WindowBase);
ferro_impl_classes!(
    TestWindowBase: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl
);

impl TestWindowBase {
    fn new() -> Ref<Self> {
        Self::with_impl(&create_mock_window_base_impl())
    }

    fn with_impl(platform_impl: &Rc<MockWindowImpl>) -> Ref<Self> {
        let platform_impl: Rc<dyn IWindowBaseImpl> = platform_impl.clone();
        instantiate(Self { base: WindowBase::construct(platform_impl) })
    }
}

/// A window base implementation without behaviour (neither a window nor a
/// top-level only: the popup kind of the double).
fn create_mock_window_base_impl() -> Rc<MockWindowImpl> {
    let platform_impl = MockWindowImpl::bare(MockImplKind::Popup);
    platform_impl.render_scaling.set(1.0);
    platform_impl
}

fn create_mock_popup_impl() -> Rc<MockWindowImpl> {
    let window_impl = MockWindowImpl::bare(MockImplKind::Popup);
    window_impl.desktop_scaling.set(1.0);
    window_impl.render_scaling.set(1.0);
    window_impl
}

fn activated(platform_impl: &MockWindowImpl) {
    IWindowBaseImpl::activated(platform_impl).expect("the activated callback is set")();
}

fn deactivated(platform_impl: &MockWindowImpl) {
    IWindowBaseImpl::deactivated(platform_impl).expect("the deactivated callback is set")();
}

fn closed(platform_impl: &MockWindowImpl) {
    ITopLevelImpl::closed(platform_impl).expect("the closed callback is set")();
}

fn is_rendering(target: &WindowBase) -> bool {
    MediaContext::instance().is_top_level_active(target.media_context_key())
}

#[test]
fn activate_should_call_impl_activate() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_window_base_impl();
    let target = TestWindowBase::with_impl(&platform_impl);

    target.activate();

    assert_eq!(1, platform_impl.count_of(&MockCall::Activate));
}

#[test]
fn impl_activate_should_call_raise_activated_event() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_window_base_impl();

    let raised = Rc::new(Cell::new(false));
    let target = TestWindowBase::with_impl(&platform_impl);
    let _ = target.activated({
        let raised = raised.clone();
        move || raised.set(true)
    });

    activated(&platform_impl);

    assert!(raised.get());
}

#[test]
fn impl_deactivate_should_call_raise_deativated_event() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let platform_impl = create_mock_window_base_impl();

    let raised = Rc::new(Cell::new(false));
    let target = TestWindowBase::with_impl(&platform_impl);
    let _ = target.deactivated({
        let raised = raised.clone();
        move || raised.set(true)
    });

    deactivated(&platform_impl);

    assert!(raised.get());
}

#[test]
fn is_visible_should_initially_be_false() {
    let _app = UnitTestApplication::start(TestServices::mock_windowing_platform());
    let target = TestWindowBase::new();

    assert!(!target.is_visible());
}

#[test]
fn is_visible_should_be_true_after_show() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::new();

    target.show();

    assert!(target.is_visible());
}

#[test]
fn is_visible_should_be_false_atfer_hide() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::new();

    target.show();
    target.hide();

    assert!(!target.is_visible());
}

#[test]
fn active_window_should_be_deactivated_when_impl_signals_close() {
    let window_impl = create_mock_popup_impl();

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::with_impl(&window_impl);
    let deactivated = Rc::new(Cell::new(0));
    let _ = target.deactivated({
        let deactivated = deactivated.clone();
        move || deactivated.set(deactivated.get() + 1)
    });

    target.show();
    activated(&window_impl);
    assert!(target.is_active());

    // Some backends (e.g. X11) never deliver a deactivation notification on close.
    closed(&window_impl);

    assert!(!target.is_active());
    assert_eq!(1, deactivated.get());
}

#[test]
fn inactive_window_should_not_raise_deactivated_when_impl_signals_close() {
    let window_impl = create_mock_popup_impl();

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::with_impl(&window_impl);
    let deactivated = Rc::new(Cell::new(0));
    let _ = target.deactivated({
        let deactivated = deactivated.clone();
        move || deactivated.set(deactivated.get() + 1)
    });

    target.show();
    assert!(!target.is_active());

    // Backends that deactivate before closing must not produce a duplicate event.
    closed(&window_impl);

    assert!(!target.is_active());
    assert_eq!(0, deactivated.get());
}

#[test]
fn is_visible_should_be_false_atfer_impl_signals_close() {
    let window_impl = create_mock_popup_impl();

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::with_impl(&window_impl);

    target.show();
    closed(&window_impl);

    assert!(!target.is_visible());
}

#[test]
fn setting_is_visible_true_shows_window() {
    let window_impl = create_mock_popup_impl();

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::with_impl(&window_impl);
    target.set_is_visible(true);

    assert_eq!(1, window_impl.count_of(&MockCall::Show { activate: true, is_dialog: false }));
}

#[test]
fn setting_is_visible_false_hides_window() {
    let window_impl = create_mock_popup_impl();

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::with_impl(&window_impl);
    target.show();
    target.set_is_visible(false);

    assert_eq!(1, window_impl.count_of(&MockCall::Hide));
}

#[test]
fn showing_should_start_renderer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::new();

    target.show();

    assert!(is_rendering(&target));
}

#[test]
fn showing_should_raise_opened() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::new();
    let raised = Rc::new(Cell::new(false));

    let _ = target.opened({
        let raised = raised.clone();
        move || raised.set(true)
    });

    target.show();

    assert!(raised.get());
}

#[test]
fn hiding_should_stop_renderer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = TestWindowBase::new();

    target.show();
    target.hide();

    assert!(!is_rendering(&target));
}

#[test]
fn renderer_should_be_disposed_when_impl_signals_close() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = create_mock_popup_impl();

    let target = TestWindowBase::with_impl(&window_impl);

    target.show();
    let renderer = target.renderer();
    closed(&window_impl);

    let renderer = renderer.as_any().downcast_ref::<NullRenderer>().expect("the test renderer is a null renderer");
    assert!(renderer.is_disposed());
}
