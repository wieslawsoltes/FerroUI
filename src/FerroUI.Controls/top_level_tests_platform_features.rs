//! Tests of the optional platform features a top-level uses: the system
//! navigation manager and the platform behavior inhibition.

use crate::platform::ITopLevelImpl;
use crate::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use crate::{PlatformInhibitionType, TopLevel, Window};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::platform::{IPlatformBehaviorInhibition, ISystemNavigationManagerImpl};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// A hand-written double of the system navigation manager.
#[derive(Default)]
struct TestSystemNavigationManager {
    back_requested: Rc<HandlerList<dyn Fn(&Rc<RoutedEventArgs>)>>,
}

impl TestSystemNavigationManager {
    fn raise_back_requested(&self) -> Rc<RoutedEventArgs> {
        let e = Rc::new(RoutedEventArgs::new());
        for (_, handler) in self.back_requested.snapshot().iter() {
            handler(&e);
        }
        e
    }
}

impl ISystemNavigationManagerImpl for TestSystemNavigationManager {
    fn back_requested(&self, handler: Rc<dyn Fn(&Rc<RoutedEventArgs>)>) -> Rc<dyn IDisposable> {
        let token = self.back_requested.add(handler);
        let handlers = self.back_requested.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
}

#[test]
fn system_navigation_back_request_should_raise_back_requested() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let navigation = Rc::new(TestSystemNavigationManager::default());
    window_impl.setup_feature::<dyn ISystemNavigationManagerImpl>(navigation.clone());
    let target = Window::with_impl(window_impl);

    let raised = Rc::new(Cell::new(0));
    target.back_requested({
        let raised = raised.clone();
        move |_, e| {
            assert!(e.routed_event().is_some_and(|event| event == *TopLevel::back_requested_event()));
            raised.set(raised.get() + 1);
        }
    });

    let e = navigation.raise_back_requested();

    assert_eq!(1, raised.get());
    assert!(e.routed_event().is_some_and(|event| event == *TopLevel::back_requested_event()));
    assert!(!e.handled());
}

#[test]
fn system_navigation_back_request_reports_handled_to_the_platform() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let navigation = Rc::new(TestSystemNavigationManager::default());
    window_impl.setup_feature::<dyn ISystemNavigationManagerImpl>(navigation.clone());
    let target = Window::with_impl(window_impl);

    target.back_requested(|_, e| e.set_handled(true));

    let e = navigation.raise_back_requested();

    assert!(e.handled());
}

#[test]
fn top_level_without_system_navigation_manager_is_created() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let target = Window::with_impl(window_impl.clone());

    assert!(ITopLevelImpl::closed(&*window_impl).is_some());
    assert!(target.platform_impl().is_some());
}

/// The completion of one request of [`TestPlatformBehaviorInhibition`].
#[derive(Default)]
struct Pending {
    completed: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl Pending {
    fn complete(&self) {
        self.completed.set(true);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

struct PendingFuture(Rc<Pending>);

impl Future for PendingFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.completed.get() {
            Poll::Ready(())
        } else {
            *self.0.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

/// A hand-written double of the platform behavior inhibition: records the
/// requests and completes them at once, or when the test says so.
struct TestPlatformBehaviorInhibition {
    complete_synchronously: bool,
    calls: RefCell<Vec<(bool, String)>>,
    pending: RefCell<Vec<Rc<Pending>>>,
}

impl TestPlatformBehaviorInhibition {
    fn new(complete_synchronously: bool) -> Rc<Self> {
        Rc::new(Self { complete_synchronously, calls: RefCell::new(Vec::new()), pending: RefCell::new(Vec::new()) })
    }

    fn complete_pending(&self) {
        for pending in self.pending.borrow_mut().drain(..) {
            pending.complete();
        }
    }
}

impl IPlatformBehaviorInhibition for TestPlatformBehaviorInhibition {
    fn set_inhibit_app_sleep(&self, inhibit_app_sleep: bool, reason: &str) -> LocalBoxFuture<()> {
        self.calls.borrow_mut().push((inhibit_app_sleep, reason.to_string()));
        let pending = Rc::new(Pending::default());
        if self.complete_synchronously {
            pending.complete();
        } else {
            self.pending.borrow_mut().push(pending.clone());
        }
        Box::pin(PendingFuture(pending))
    }
}

#[test]
fn request_platform_inhibition_without_the_feature_has_no_effect() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Window::with_impl(MockWindowingPlatform::create_window_mock());

    let task = target.request_platform_inhibition(PlatformInhibitionType::AppSleep, "Test");

    assert!(task.is_completed_successfully());
    task.result().expect("the task has completed").dispose();
}

#[test]
fn request_platform_inhibition_inhibits_app_sleep_until_disposed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let inhibition = TestPlatformBehaviorInhibition::new(true);
    window_impl.setup_feature::<dyn IPlatformBehaviorInhibition>(inhibition.clone());
    let target = Window::with_impl(window_impl);

    let task = target.request_platform_inhibition(PlatformInhibitionType::AppSleep, "Playing");

    assert_eq!(vec![(true, "Playing".to_string())], *inhibition.calls.borrow());
    assert!(task.is_completed_successfully());

    let inhibited = task.result().expect("the task has completed");
    assert_eq!(1, inhibition.calls.borrow().len());

    inhibited.dispose();
    assert_eq!(vec![(true, "Playing".to_string()), (false, "Playing".to_string())], *inhibition.calls.borrow());

    // Disposing again does nothing.
    inhibited.dispose();
    assert_eq!(2, inhibition.calls.borrow().len());
}

#[test]
fn request_platform_inhibition_completes_when_the_platform_has_applied_it() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window_impl = MockWindowingPlatform::create_window_mock();
    let inhibition = TestPlatformBehaviorInhibition::new(false);
    window_impl.setup_feature::<dyn IPlatformBehaviorInhibition>(inhibition.clone());
    let target = Window::with_impl(window_impl);

    let task = target.request_platform_inhibition(PlatformInhibitionType::AppSleep, "Playing");

    // The request is sent before the method returns.
    assert_eq!(vec![(true, "Playing".to_string())], *inhibition.calls.borrow());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!task.is_completed());

    inhibition.complete_pending();
    assert!(!task.is_completed());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(task.is_completed_successfully());

    // Lifting the inhibition does not wait for the platform.
    let inhibited = task.result().expect("the task has completed");
    inhibited.dispose();
    assert_eq!(vec![(true, "Playing".to_string()), (false, "Playing".to_string())], *inhibition.calls.borrow());
    assert_eq!(1, inhibition.pending.borrow().len());

    inhibition.complete_pending();
    Dispatcher::ui_thread().run_jobs(None);
}
