use std::cell::{Cell, RefCell};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use super::dispatcher_tests_exception::{message_of, DispatcherTests};
use super::{
    CancellationToken, CancellationTokenSource, Dispatcher, DispatcherFrame, DispatcherImplEvent, DispatcherOperation,
    DispatcherInvokeError, DispatcherOperationStatus, DispatcherPriority, DispatcherTask, DispatcherTaskScheduler,
    DispatcherTimer, FerroSynchronizationContext, IControlledDispatcherImpl, IDispatcher, IDispatcherImpl,
    IDispatcherImplWithExplicitBackgroundProcessing, IDispatcherImplWithPendingInput, IDispatcherSignal,
    OperationCanceledError,
};
use crate::platform::ManagedDispatcherImpl;
use crate::reactive::IDisposable;
use crate::FerroLocator;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SimpleKind {
    Simple,
    WithBackgroundProcessing,
    Controlled,
}

#[derive(Default)]
struct SimpleState {
    next_timer: Option<i64>,
    asked_for_signal: bool,
    now: i64,
    test_input_pending: Option<bool>,
    asked_for_background_processing: bool,
    run_loop_count: i32,
}

/// The simulated platform implementation the tests drive by hand. The three
/// classes of the reference test-suite (simple, with background processing,
/// controlled) are the three kinds of this type.
struct SimpleDispatcherImpl {
    kind: SimpleKind,
    loop_thread: ThreadId,
    state: Arc<Mutex<SimpleState>>,
    signaled: DispatcherImplEvent,
    timer: DispatcherImplEvent,
    ready_for_background_processing: DispatcherImplEvent,
}

impl SimpleDispatcherImpl {
    fn new(kind: SimpleKind) -> Rc<Self> {
        Self::new_on_thread(kind, thread::current().id())
    }

    fn new_on_thread(kind: SimpleKind, loop_thread: ThreadId) -> Rc<Self> {
        Rc::new(Self {
            kind,
            loop_thread,
            state: Arc::new(Mutex::new(SimpleState::default())),
            signaled: DispatcherImplEvent::new(),
            timer: DispatcherImplEvent::new(),
            ready_for_background_processing: DispatcherImplEvent::new(),
        })
    }

    fn next_timer(&self) -> Option<i64> {
        self.state.lock().unwrap().next_timer
    }

    fn asked_for_signal(&self) -> bool {
        self.state.lock().unwrap().asked_for_signal
    }

    fn set_now(&self, now: i64) {
        self.state.lock().unwrap().now = now;
    }

    fn add_now(&self, delta: i64) {
        self.state.lock().unwrap().now += delta;
    }

    fn set_test_input_pending(&self, value: Option<bool>) {
        self.state.lock().unwrap().test_input_pending = value;
    }

    fn asked_for_background_processing(&self) -> bool {
        self.state.lock().unwrap().asked_for_background_processing
    }

    fn run_loop_count(&self) -> i32 {
        self.state.lock().unwrap().run_loop_count
    }

    fn execute_signal(&self) {
        {
            let mut state = self.state.lock().unwrap();
            if !state.asked_for_signal {
                return;
            }
            state.asked_for_signal = false;
        }
        self.signaled.raise();
    }

    fn execute_timer(&self) {
        {
            let mut state = self.state.lock().unwrap();
            let Some(next_timer) = state.next_timer else {
                return;
            };
            state.now = next_timer;
        }
        self.timer.raise();
    }

    fn fire_background_processing(&self) {
        {
            let mut state = self.state.lock().unwrap();
            if !state.asked_for_background_processing {
                return;
            }
            state.asked_for_background_processing = false;
        }
        self.ready_for_background_processing.raise();
    }
}

/// The cross-thread half of [`SimpleDispatcherImpl`].
struct SimpleSignal(Arc<Mutex<SimpleState>>);

impl IDispatcherSignal for SimpleSignal {
    fn signal(&self) {
        self.0.lock().unwrap().asked_for_signal = true;
    }
}

impl IDispatcherImpl for SimpleDispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool {
        thread::current().id() == self.loop_thread
    }

    fn signal(&self) {
        self.state.lock().unwrap().asked_for_signal = true;
    }

    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        Arc::new(SimpleSignal(self.state.clone()))
    }

    fn signaled(&self) -> &DispatcherImplEvent {
        &self.signaled
    }

    fn timer(&self) -> &DispatcherImplEvent {
        &self.timer
    }

    fn now(&self) -> i64 {
        self.state.lock().unwrap().now
    }

    fn update_timer(&self, due_time_in_ms: Option<i64>) {
        self.state.lock().unwrap().next_timer = due_time_in_ms;
    }

    fn as_pending_input(&self) -> Option<&dyn IDispatcherImplWithPendingInput> {
        Some(self)
    }

    fn as_explicit_background_processing(&self) -> Option<&dyn IDispatcherImplWithExplicitBackgroundProcessing> {
        match self.kind {
            SimpleKind::Simple => None,
            _ => Some(self),
        }
    }

    fn as_controlled(&self) -> Option<&dyn IControlledDispatcherImpl> {
        match self.kind {
            SimpleKind::Controlled => Some(self),
            _ => None,
        }
    }
}

impl IDispatcherImplWithPendingInput for SimpleDispatcherImpl {
    fn can_query_pending_input(&self) -> bool {
        self.state.lock().unwrap().test_input_pending.is_some()
    }

    fn has_pending_input(&self) -> bool {
        self.state.lock().unwrap().test_input_pending == Some(true)
    }
}

impl IDispatcherImplWithExplicitBackgroundProcessing for SimpleDispatcherImpl {
    fn ready_for_background_processing(&self) -> &DispatcherImplEvent {
        &self.ready_for_background_processing
    }

    fn request_background_processing(&self) {
        if !self.current_thread_is_loop_thread() {
            panic!("request_background_processing called from a foreign thread");
        }
        self.state.lock().unwrap().asked_for_background_processing = true;
    }
}

impl IControlledDispatcherImpl for SimpleDispatcherImpl {
    fn run_loop(&self, token: CancellationToken) {
        self.state.lock().unwrap().run_loop_count += 1;
        let st = Instant::now();
        while !token.is_cancellation_requested() {
            self.fire_background_processing();
            self.execute_signal();
            assert!(st.elapsed() < Duration::from_millis(4000), "run_loop exceeded test time quota");
        }
    }
}

fn names(actions: &Rc<RefCell<Vec<String>>>) -> Vec<String> {
    actions.borrow().clone()
}

fn add(actions: &Rc<RefCell<Vec<String>>>, name: &'static str) -> impl FnOnce() + 'static {
    let actions = actions.clone();
    move || actions.borrow_mut().push(name.to_string())
}

/// Cancels the returned source after `timeout`, as a safety net for loops.
fn cancel_after(timeout: Duration) -> CancellationTokenSource {
    let source = CancellationTokenSource::new();
    let to_cancel = source.clone();
    thread::spawn(move || {
        thread::sleep(timeout);
        to_cancel.cancel();
    });
    source
}

#[test]
fn dispatcher_executes_jobs_according_to_priority() {
    let t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());
    let actions = Rc::new(RefCell::new(Vec::new()));
    t.ui_thread.post_local(add(&actions, "Background"), DispatcherPriority::BACKGROUND);
    t.ui_thread.post_local(add(&actions, "Render"), DispatcherPriority::RENDER);
    t.ui_thread.post_local(add(&actions, "Input"), DispatcherPriority::INPUT);
    assert!(impl_.asked_for_signal());
    impl_.execute_signal();
    assert_eq!(names(&actions), ["Render", "Input", "Background"]);
}

#[test]
fn dispatcher_preserves_order_when_changing_priority() {
    let t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());
    let actions = Rc::new(RefCell::new(Vec::new()));
    let to_promote =
        t.ui_thread.invoke_async_local_with_priority(add(&actions, "PromotedRender"), DispatcherPriority::BACKGROUND);
    let to_promote2 =
        t.ui_thread.invoke_async_local_with_priority(add(&actions, "PromotedRender2"), DispatcherPriority::INPUT);
    t.ui_thread.post_local(add(&actions, "Render"), DispatcherPriority::RENDER);
    to_promote.set_priority(DispatcherPriority::RENDER);
    to_promote2.set_priority(DispatcherPriority::RENDER);

    assert!(impl_.asked_for_signal());
    impl_.execute_signal();

    assert_eq!(names(&actions), ["PromotedRender", "PromotedRender2", "Render"]);
}

#[test]
fn dispatcher_repeats_background_processing_request_to_the_new_implementation() {
    let t = DispatcherTests::new();
    let actions = Rc::new(RefCell::new(Vec::new()));

    // Requests background processing from the pre-initialization implementation
    t.ui_thread.post_local(add(&actions, "Background"), DispatcherPriority::BACKGROUND);

    let impl_ = SimpleDispatcherImpl::new(SimpleKind::WithBackgroundProcessing);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());

    assert!(impl_.asked_for_background_processing());
    impl_.fire_background_processing();
    assert_eq!(names(&actions), ["Background"]);
}

#[test]
fn dispatcher_moves_background_processing_request_to_the_timer_of_the_new_implementation() {
    // Not in the reference suite: the new implementation has no explicit
    // background processing, so the pending request falls back to the timer.
    let t = DispatcherTests::new();
    let actions = Rc::new(RefCell::new(Vec::new()));
    t.ui_thread.post_local(add(&actions, "Background"), DispatcherPriority::BACKGROUND);

    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    impl_.set_now(1000);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());

    assert_eq!(impl_.next_timer(), Some(1001));
    impl_.execute_timer();
    assert_eq!(names(&actions), ["Background"]);
    assert_eq!(impl_.next_timer(), None);
}

#[test]
fn dispatcher_repeats_signal_to_the_new_implementation() {
    let t = DispatcherTests::new();
    let actions = Rc::new(RefCell::new(Vec::new()));

    // Signals the pre-initialization implementation
    t.ui_thread.post_local(add(&actions, "Render"), DispatcherPriority::RENDER);

    let impl_ = SimpleDispatcherImpl::new(SimpleKind::WithBackgroundProcessing);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());

    assert!(impl_.asked_for_signal());
    impl_.execute_signal();
    assert_eq!(names(&actions), ["Render"]);
}

#[test]
fn dispatcher_stops_item_processing_when_interactivity_deadline_is_reached() {
    let _t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::reset_for_unit_tests();
    let ui_thread = Dispatcher::new(Some(impl_.clone()));
    let actions = Rc::new(RefCell::new(Vec::new()));
    for c in 0..10 {
        let item_id = c;
        let actions = actions.clone();
        let impl_ = impl_.clone();
        ui_thread.post_local(
            move || {
                actions.borrow_mut().push(item_id);
                impl_.add_now(20);
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    assert!(!impl_.asked_for_signal());
    assert!(impl_.next_timer().is_some());

    for c in 0..4 {
        assert!(impl_.next_timer().is_some());
        assert!(!impl_.asked_for_signal());
        impl_.execute_timer();
        assert!(!impl_.asked_for_signal());
        impl_.execute_signal();
        let mut expected_count = (c + 1) * 3;
        if c == 3 {
            expected_count = 10;
        }

        assert_eq!(*actions.borrow(), (0..expected_count).collect::<Vec<_>>());
        assert!(!impl_.asked_for_signal());
        if c < 3 {
            assert!(impl_.next_timer().unwrap() > impl_.now());
        } else {
            assert!(impl_.next_timer().is_none());
        }
    }
}

#[test]
fn dispatcher_stops_item_processing_when_input_is_pending() {
    let _t = DispatcherTests::new();
    Dispatcher::reset_for_unit_tests();

    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    impl_.set_test_input_pending(Some(true));
    let ui_thread = Dispatcher::new(Some(impl_.clone()));

    let actions = Rc::new(RefCell::new(Vec::new()));
    for c in 0..10 {
        let item_id = c;
        let actions = actions.clone();
        let impl_ = impl_.clone();
        ui_thread.post_local(
            move || {
                actions.borrow_mut().push(item_id);
                if item_id == 0 || item_id == 3 || item_id == 7 {
                    impl_.set_test_input_pending(Some(true));
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    assert!(!impl_.asked_for_signal());
    assert!(impl_.next_timer().is_some());
    impl_.set_test_input_pending(Some(false));

    for c in 0..4 {
        assert!(impl_.next_timer().is_some());
        impl_.execute_timer();
        assert!(!impl_.asked_for_signal());
        let expected_count = match c {
            0 => 1,
            1 => 4,
            2 => 8,
            3 => 10,
            _ => unreachable!(),
        };

        assert_eq!(*actions.borrow(), (0..expected_count).collect::<Vec<_>>());
        assert!(!impl_.asked_for_signal());
        if c < 3 {
            assert!(impl_.next_timer().unwrap() > impl_.now());
            impl_.set_now(impl_.next_timer().unwrap() + 1);
        } else {
            assert!(impl_.next_timer().is_none());
        }

        impl_.set_test_input_pending(Some(false));
    }
}

fn can_wait_for_dispatcher_operation_from_the_same_thread(controlled: bool, foreground: bool) {
    let t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(if controlled { SimpleKind::Controlled } else { SimpleKind::Simple });
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());
    let finished = Rc::new(Cell::new(false));

    let f = finished.clone();
    t.ui_thread
        .invoke_async_local_with_priority(
            move || f.set(true),
            if foreground { DispatcherPriority::DEFAULT } else { DispatcherPriority::BACKGROUND },
        )
        .wait()
        .unwrap();

    assert!(finished.get());
    if controlled {
        assert_eq!(impl_.run_loop_count(), if foreground { 0 } else { 1 });
    }
}

#[test]
fn can_wait_for_dispatcher_operation_from_the_same_thread_simple_background() {
    can_wait_for_dispatcher_operation_from_the_same_thread(false, false);
}

#[test]
fn can_wait_for_dispatcher_operation_from_the_same_thread_simple_foreground() {
    can_wait_for_dispatcher_operation_from_the_same_thread(false, true);
}

#[test]
fn can_wait_for_dispatcher_operation_from_the_same_thread_controlled_background() {
    can_wait_for_dispatcher_operation_from_the_same_thread(true, false);
}

#[test]
fn can_wait_for_dispatcher_operation_from_the_same_thread_controlled_foreground() {
    can_wait_for_dispatcher_operation_from_the_same_thread(true, true);
}

/// Scoped services with a freshly initialized UI thread dispatcher.
struct DispatcherServices {
    scope: Rc<dyn IDisposable>,
}

impl DispatcherServices {
    fn new(impl_: Rc<dyn IDispatcherImpl>) -> Self {
        let scope = FerroLocator::enter_scope();
        Dispatcher::reset_for_unit_tests();
        Dispatcher::initialize_ui_thread_dispatcher(impl_);
        Self { scope }
    }
}

impl Drop for DispatcherServices {
    fn drop(&mut self) {
        if !thread::panicking() {
            Dispatcher::reset_for_unit_tests();
        }
        self.scope.dispose();
    }
}

#[test]
fn dispatcher_frame_uses_current_dispatcher() {
    let _t = DispatcherTests::new();
    let ui_thread_dispatcher = Dispatcher::ui_thread();

    thread::spawn(move || {
        let current_dispatcher = Dispatcher::current_dispatcher();
        let frame = DispatcherFrame::new();

        assert!(!Arc::ptr_eq(&ui_thread_dispatcher, &current_dispatcher));
        assert!(Arc::ptr_eq(&current_dispatcher, frame.dispatcher()));
    })
    .join()
    .unwrap();
}

#[test]
fn exit_all_frames_should_exit_all_frames_and_be_able_to_continue() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let disp = Dispatcher::ui_thread();
    {
        let actions = actions.clone();
        disp.post_local(
            move || {
                actions.borrow_mut().push("Nested frame".to_string());
                Dispatcher::ui_thread().main_loop(&CancellationToken::none());
                actions.borrow_mut().push("Nested frame exited".to_string());
            },
            DispatcherPriority::DEFAULT,
        );
    }
    {
        let actions = actions.clone();
        let d = disp.clone();
        disp.post_local(
            move || {
                actions.borrow_mut().push("ExitAllFrames".to_string());
                d.exit_all_frames();
            },
            DispatcherPriority::DEFAULT,
        );
    }

    disp.main_loop(&CancellationToken::none());

    assert_eq!(names(&actions), ["Nested frame", "ExitAllFrames", "Nested frame exited"]);
    actions.borrow_mut().clear();

    let second_loop = CancellationTokenSource::new();
    {
        let actions = actions.clone();
        let second_loop = second_loop.clone();
        disp.post_local(
            move || {
                actions.borrow_mut().push("Callback after exit".to_string());
                second_loop.cancel();
            },
            DispatcherPriority::DEFAULT,
        );
    }
    disp.main_loop(&second_loop.token());
    assert_eq!(names(&actions), ["Callback after exit"]);
}

#[test]
fn shutdown_should_exit_all_frames_and_not_allow_new_frames() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let disp = Dispatcher::ui_thread();
    {
        let actions = actions.clone();
        disp.post_local(
            move || {
                actions.borrow_mut().push("Nested frame".to_string());
                Dispatcher::ui_thread().main_loop(&CancellationToken::none());
                actions.borrow_mut().push("Nested frame exited".to_string());
            },
            DispatcherPriority::DEFAULT,
        );
    }

    let critical_frame = DispatcherFrame::with_exit_when_requested(false);
    {
        let actions = actions.clone();
        let critical_frame = critical_frame.clone();
        disp.post_local(
            move || {
                actions.borrow_mut().push("Critical frame".to_string());
                Dispatcher::ui_thread().push_frame(&critical_frame);
                actions.borrow_mut().push("Critical frame exited".to_string());
            },
            DispatcherPriority::DEFAULT,
        );
    }
    {
        let actions = actions.clone();
        let d = disp.clone();
        disp.post_local(
            move || {
                actions.borrow_mut().push("Shutdown".to_string());
                d.begin_invoke_shutdown(DispatcherPriority::NORMAL);
            },
            DispatcherPriority::DEFAULT,
        );
    }
    {
        let actions = actions.clone();
        disp.post_local(
            move || {
                actions.borrow_mut().push("Nested frame after shutdown".to_string());
                // This should exit immediately and not run any jobs
                Dispatcher::ui_thread().main_loop(&CancellationToken::none());
                actions.borrow_mut().push("Nested frame after shutdown exited".to_string());
            },
            DispatcherPriority::DEFAULT,
        );
    }
    disp.post_local(add(&actions, "Job in critical frame"), DispatcherPriority::DEFAULT);
    {
        let actions = actions.clone();
        let critical_frame = critical_frame.clone();
        disp.post_local(
            move || {
                actions.borrow_mut().push("Stop critical frame".to_string());
                critical_frame.set_continue(false);
            },
            DispatcherPriority::DEFAULT,
        );
    }

    disp.main_loop(&CancellationToken::none());

    assert_eq!(
        names(&actions),
        [
            "Nested frame",
            "Critical frame",
            "Shutdown",
            // Normal nested frames are supposed to exit immediately
            "Nested frame after shutdown",
            "Nested frame after shutdown exited",
            // if frame is configured to not answer dispatcher requests, it should be allowed to run
            "Job in critical frame",
            "Stop critical frame",
            "Critical frame exited",
            // After 3-rd level frames have exited, the normal nested frame exits too
            "Nested frame exited"
        ]
    );
    actions.borrow_mut().clear();

    disp.post_local(add(&actions, "Frame after shutdown finished"), DispatcherPriority::DEFAULT);
    let d = disp.clone();
    let result = catch_unwind(AssertUnwindSafe(|| d.main_loop(&CancellationToken::none())));
    assert_eq!(
        message_of(&*result.unwrap_err()),
        Some("Cannot perform requested operation because the Dispatcher shut down")
    );
    assert!(actions.borrow().is_empty());
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TestDispatcherImplKind {
    Simple,
    SimpleWithBackgroundProcessing,
    SimpleControlled,
    Managed,
}

enum TestImpl {
    Simple(Rc<SimpleDispatcherImpl>),
    Managed(Rc<ManagedDispatcherImpl>),
}

impl TestImpl {
    fn as_impl(&self) -> Rc<dyn IDispatcherImpl> {
        match self {
            TestImpl::Simple(impl_) => impl_.clone(),
            TestImpl::Managed(impl_) => impl_.clone(),
        }
    }
}

fn create_dispatcher_impl(kind: TestDispatcherImplKind) -> TestImpl {
    match kind {
        TestDispatcherImplKind::Simple => TestImpl::Simple(SimpleDispatcherImpl::new(SimpleKind::Simple)),
        TestDispatcherImplKind::SimpleWithBackgroundProcessing => {
            TestImpl::Simple(SimpleDispatcherImpl::new(SimpleKind::WithBackgroundProcessing))
        }
        TestDispatcherImplKind::SimpleControlled => TestImpl::Simple(SimpleDispatcherImpl::new(SimpleKind::Controlled)),
        TestDispatcherImplKind::Managed => TestImpl::Managed(Rc::new(ManagedDispatcherImpl::new(None))),
    }
}

fn shutdown_from_operation_aborts_queued_operations_without_running_them(kind: TestDispatcherImplKind) {
    let _t = DispatcherTests::new();
    let impl_ = create_dispatcher_impl(kind);
    let _services = DispatcherServices::new(impl_.as_impl());
    let disp = Dispatcher::ui_thread();

    let actions = Rc::new(RefCell::new(Vec::new()));
    {
        let actions = actions.clone();
        disp.shutdown_finished(move |_| actions.borrow_mut().push("ShutdownFinished".to_string()));
    }

    let op1 = {
        let actions = actions.clone();
        let d = disp.clone();
        disp.invoke_async_local_with_priority(
            move || {
                actions.borrow_mut().push("op1".to_string());
                d.invoke_shutdown();
            },
            DispatcherPriority::SEND,
        )
    };
    let op2 = disp.invoke_async_local_with_priority(add(&actions, "op2"), DispatcherPriority::SEND);

    if disp.supports_run_loops() {
        let timeout = cancel_after(Duration::from_secs(5));
        disp.main_loop(&timeout.token());
    } else {
        let TestImpl::Simple(simple) = &impl_ else { unreachable!() };
        assert!(simple.asked_for_signal());
        simple.execute_signal();
    }

    assert_eq!(names(&actions), ["op1", "ShutdownFinished"]);
    assert_eq!(op1.status(), DispatcherOperationStatus::Completed);
    assert_eq!(op2.status(), DispatcherOperationStatus::Aborted);
    assert_eq!(op2.wait(), Err(OperationCanceledError));
}

#[test]
fn shutdown_from_operation_aborts_queued_operations_without_running_them_simple() {
    shutdown_from_operation_aborts_queued_operations_without_running_them(TestDispatcherImplKind::Simple);
}

#[test]
fn shutdown_from_operation_aborts_queued_operations_without_running_them_simple_with_background_processing() {
    shutdown_from_operation_aborts_queued_operations_without_running_them(
        TestDispatcherImplKind::SimpleWithBackgroundProcessing,
    );
}

#[test]
fn shutdown_from_operation_aborts_queued_operations_without_running_them_simple_controlled() {
    shutdown_from_operation_aborts_queued_operations_without_running_them(TestDispatcherImplKind::SimpleControlled);
}

#[test]
fn shutdown_from_operation_aborts_queued_operations_without_running_them_managed() {
    shutdown_from_operation_aborts_queued_operations_without_running_them(TestDispatcherImplKind::Managed);
}

fn operations_are_only_aborted_after_last_frame_exits(kind: TestDispatcherImplKind) {
    let _t = DispatcherTests::new();
    let impl_ = create_dispatcher_impl(kind);
    let _services = DispatcherServices::new(impl_.as_impl());
    let disp = Dispatcher::ui_thread();

    let actions = Rc::new(RefCell::new(Vec::new()));
    {
        let actions = actions.clone();
        disp.shutdown_finished(move |_| actions.borrow_mut().push("ShutdownFinished".to_string()));
    }

    let critical_frame = DispatcherFrame::with_exit_when_requested(false);
    let op2: Rc<RefCell<Option<DispatcherOperation>>> = Rc::new(RefCell::new(None));
    let op3: Rc<RefCell<Option<DispatcherOperation>>> = Rc::new(RefCell::new(None));
    let op4: Rc<RefCell<Option<DispatcherOperation>>> = Rc::new(RefCell::new(None));
    {
        let actions = actions.clone();
        let d = disp.clone();
        let critical_frame = critical_frame.clone();
        let (op2, op3, op4) = (op2.clone(), op3.clone(), op4.clone());
        disp.invoke_async_local_with_priority(
            move || {
                actions.borrow_mut().push("op1".to_string());
                d.invoke_shutdown();

                let op2 = op2.borrow().clone().unwrap();
                let op3 = op3.borrow().clone().unwrap();

                // Frames are still on the stack, so nothing has been aborted yet
                assert_eq!(op2.status(), DispatcherOperationStatus::Pending);
                assert_eq!(op3.status(), DispatcherOperationStatus::Pending);

                // Explicit run_jobs and synchronous invoke still dispatch pending operations
                d.run_jobs(Some(DispatcherPriority::NORMAL));
                assert_eq!(op2.status(), DispatcherOperationStatus::Completed);
                assert_eq!(op3.status(), DispatcherOperationStatus::Pending);
                d.invoke_local_with_priority(add(&actions, "invoke"), DispatcherPriority::NORMAL).unwrap();
                assert_eq!(op3.status(), DispatcherOperationStatus::Pending);

                // A frame that ignores exit requests still pumps the remaining operations
                d.push_frame(&critical_frame);
                assert_eq!(op3.status(), DispatcherOperationStatus::Completed);

                let new_op4 = d.invoke_async_local_with_priority(add(&actions, "op4"), DispatcherPriority::NORMAL);
                assert_eq!(new_op4.status(), DispatcherOperationStatus::Pending);
                *op4.borrow_mut() = Some(new_op4);
            },
            DispatcherPriority::NORMAL,
        );
    }
    *op2.borrow_mut() = Some(disp.invoke_async_local_with_priority(add(&actions, "op2"), DispatcherPriority::NORMAL));
    *op3.borrow_mut() = Some({
        let actions = actions.clone();
        let critical_frame = critical_frame.clone();
        disp.invoke_async_local_with_priority(
            move || {
                actions.borrow_mut().push("op3".to_string());
                critical_frame.set_continue(false);
            },
            DispatcherPriority::BACKGROUND,
        )
    });

    let timeout = cancel_after(Duration::from_secs(5));
    disp.main_loop(&timeout.token());

    assert_eq!(names(&actions), ["op1", "op2", "invoke", "op3", "ShutdownFinished"]);
    assert_eq!(op4.borrow().as_ref().unwrap().status(), DispatcherOperationStatus::Aborted);
}

#[test]
fn operations_are_only_aborted_after_last_frame_exits_simple_controlled() {
    operations_are_only_aborted_after_last_frame_exits(TestDispatcherImplKind::SimpleControlled);
}

#[test]
fn operations_are_only_aborted_after_last_frame_exits_managed() {
    operations_are_only_aborted_after_last_frame_exits(TestDispatcherImplKind::Managed);
}

#[test]
fn disable_processing_should_stop_processing() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));
    let disp = Dispatcher::ui_thread();
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    disp.post_local(move || r.set(true), DispatcherPriority::NORMAL);

    let disabled = disp.disable_processing();
    let d = disp.clone();
    let result = catch_unwind(AssertUnwindSafe(|| d.main_loop(&CancellationToken::none())));
    assert_eq!(
        message_of(&*result.unwrap_err()),
        Some("Cannot perform this operation while dispatcher processing is suspended.")
    );
    let d = disp.clone();
    let result = catch_unwind(AssertUnwindSafe(|| d.run_jobs(None)));
    assert_eq!(
        message_of(&*result.unwrap_err()),
        Some("Cannot perform this operation while dispatcher processing is suspended.")
    );
    assert!(!ran.get());

    disabled.dispose();
    // Disposing twice does not unbalance the counter.
    disabled.dispose();
    disp.run_jobs(None);
    assert!(ran.get());
}

// ---------------------------------------------------------------------------
// Tests that are not in the reference suite: the Send/local split, waiting
// across threads, futures, timers and the remaining public surface.
// ---------------------------------------------------------------------------

struct CountingWaker {
    wakes: AtomicUsize,
    woken_on: Mutex<Option<ThreadId>>,
    on_wake: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

impl CountingWaker {
    fn new() -> Arc<Self> {
        Arc::new(Self { wakes: AtomicUsize::new(0), woken_on: Mutex::new(None), on_wake: Mutex::new(None) })
    }
}

impl Wake for CountingWaker {
    fn wake(self: Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
        *self.woken_on.lock().unwrap() = Some(thread::current().id());
        let on_wake = self.on_wake.lock().unwrap().take();
        if let Some(on_wake) = on_wake {
            on_wake();
        }
    }
}

fn poll_once<F: Future + Unpin>(future: &mut F, waker: &Arc<CountingWaker>) -> Poll<F::Output> {
    let waker = Waker::from(waker.clone());
    let mut cx = Context::from_waker(&waker);
    Pin::new(future).poll(&mut cx)
}

#[test]
fn post_from_another_thread_runs_on_the_dispatcher_thread() {
    let t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());

    let ran_on = Arc::new(Mutex::new(None));
    let dispatcher = t.ui_thread.clone();
    let r = ran_on.clone();
    thread::spawn(move || {
        assert!(!dispatcher.check_access());
        // Cross-thread posts always ask for foreground processing, whatever the priority.
        dispatcher.post(move || *r.lock().unwrap() = Some(thread::current().id()), DispatcherPriority::BACKGROUND);
    })
    .join()
    .unwrap();

    assert!(impl_.asked_for_signal());
    assert!(ran_on.lock().unwrap().is_none());
    impl_.execute_signal();
    // The job is a background job: the signal handler defers it to the timer.
    impl_.execute_timer();
    assert_eq!(*ran_on.lock().unwrap(), Some(thread::current().id()));
}

#[test]
fn post_through_the_dispatcher_interface() {
    let t = DispatcherTests::new();
    let dispatcher: Arc<dyn IDispatcher> = t.ui_thread.clone();
    let ran = Arc::new(AtomicBool::new(false));
    let r = ran.clone();
    dispatcher.post(Box::new(move || r.store(true, Ordering::SeqCst)), DispatcherPriority::NORMAL);
    assert!(dispatcher.check_access());
    dispatcher.verify_access();
    t.ui_thread.run_jobs(None);
    assert!(ran.load(Ordering::SeqCst));
}

#[test]
fn local_methods_panic_on_other_threads() {
    let t = DispatcherTests::new();
    let dispatcher = t.ui_thread.clone();
    let message = thread::spawn(move || {
        let result = catch_unwind(AssertUnwindSafe(|| dispatcher.post_local(|| {}, DispatcherPriority::NORMAL)));
        message_of(&*result.unwrap_err()).map(str::to_string)
    })
    .join()
    .unwrap();
    assert_eq!(
        message.as_deref(),
        Some("The calling thread cannot access this object because a different thread owns it.")
    );
    assert!(!t.ui_thread.has_jobs_with_priority(DispatcherPriority::INACTIVE));
}

#[test]
fn invoke_from_another_thread_blocks_until_the_dispatcher_ran_the_callback() {
    let t = DispatcherTests::new();
    let dispatcher = t.ui_thread.clone();
    let ui_thread_id = thread::current().id();
    let done = CancellationTokenSource::new();
    let finish = done.clone();
    let worker = thread::spawn(move || {
        let result = dispatcher.invoke(move || (thread::current().id(), 42)).unwrap();
        let op = dispatcher.invoke_async(|| "async".to_string());
        let async_result = op.result().unwrap();
        finish.cancel();
        (result, async_result)
    });

    let timeout = cancel_after(Duration::from_secs(10));
    let stop = timeout.clone();
    done.token().register(move || stop.cancel());
    t.ui_thread.main_loop(&timeout.token());

    let ((ran_on, value), async_result) = worker.join().unwrap();
    assert_eq!(ran_on, ui_thread_id);
    assert_eq!(value, 42);
    assert_eq!(async_result, "async");
}

#[test]
fn invoke_on_the_dispatcher_thread_at_send_priority_runs_inline() {
    let t = DispatcherTests::new();
    assert_eq!(t.ui_thread.invoke(|| 5), Ok(5));
    let value = Rc::new(Cell::new(1));
    let v = value.clone();
    assert_eq!(t.ui_thread.invoke_local(move || v.replace(2)), Ok(1));
    assert_eq!(value.get(), 2);
    assert!(!t.ui_thread.has_jobs_with_priority(DispatcherPriority::INACTIVE));

    // Other priorities go through the queue and pump it.
    let order = Rc::new(RefCell::new(Vec::new()));
    t.ui_thread.post_local(add(&order, "posted"), DispatcherPriority::NORMAL);
    let o = order.clone();
    let result = t.ui_thread.invoke_local_with_priority(
        move || {
            o.borrow_mut().push("invoked".to_string());
            Rc::new(7)
        },
        DispatcherPriority::NORMAL,
    );
    assert_eq!(*result.unwrap(), 7);
    assert_eq!(names(&order), ["posted", "invoked"]);
}

#[test]
fn invoke_with_canceled_token_does_not_run() {
    let t = DispatcherTests::new();
    let source = CancellationTokenSource::new();
    source.cancel();
    let ran = Arc::new(AtomicBool::new(false));
    let r = ran.clone();
    let result = t.ui_thread.invoke_with_cancellation(
        move || r.store(true, Ordering::SeqCst),
        DispatcherPriority::SEND,
        &source.token(),
    );
    assert_eq!(result, Err(OperationCanceledError));
    let r = ran.clone();
    let op = t.ui_thread.invoke_async_with_cancellation(
        move || r.store(true, Ordering::SeqCst),
        DispatcherPriority::NORMAL,
        &source.token(),
    );
    assert_eq!(op.status(), DispatcherOperationStatus::Aborted);
    t.ui_thread.run_jobs(None);
    assert!(!ran.load(Ordering::SeqCst));
}

#[test]
fn cancellation_aborts_a_pending_operation() {
    let t = DispatcherTests::new();
    let source = CancellationTokenSource::new();
    let ran = Rc::new(Cell::new(false));
    let aborted = Arc::new(AtomicBool::new(false));
    let r = ran.clone();
    let op =
        t.ui_thread.invoke_async_local_with_cancellation(move || r.set(true), DispatcherPriority::NORMAL, &source.token());
    let a = aborted.clone();
    op.aborted(move || a.store(true, Ordering::SeqCst));
    assert_eq!(op.status(), DispatcherOperationStatus::Pending);

    source.cancel();
    assert_eq!(op.status(), DispatcherOperationStatus::Aborted);
    assert!(aborted.load(Ordering::SeqCst));
    assert!(!op.abort());
    t.ui_thread.run_jobs(None);
    assert!(!ran.get());
    assert_eq!(op.result(), Err(OperationCanceledError));
}

#[test]
fn operation_events_and_result() {
    let t = DispatcherTests::new();
    let completed = Arc::new(AtomicUsize::new(0));
    let op = t.ui_thread.invoke_async_local(|| Rc::new("value".to_string()));
    let c = completed.clone();
    op.completed(move || {
        c.fetch_add(1, Ordering::SeqCst);
    });
    let c = completed.clone();
    let removed = op.completed(move || {
        c.fetch_add(10, Ordering::SeqCst);
    });
    removed.dispose();
    assert_eq!(op.priority(), DispatcherPriority::DEFAULT);
    assert!(Arc::ptr_eq(op.dispatcher(), &t.ui_thread));

    t.ui_thread.run_jobs(None);
    assert_eq!(op.status(), DispatcherOperationStatus::Completed);
    assert_eq!(completed.load(Ordering::SeqCst), 1);
    assert_eq!(*op.result().unwrap(), "value");
    assert!(!op.abort());
}

#[test]
fn local_operation_storage_is_released() {
    let t = DispatcherTests::new();
    let local = t.ui_thread.try_local().unwrap();
    let marker = Rc::new(());

    // Result never read.
    let m = marker.clone();
    let op = t.ui_thread.invoke_async_local(move || m);
    t.ui_thread.run_jobs(None);
    assert_eq!(Rc::strong_count(&marker), 2);
    drop(op);
    assert_eq!(Rc::strong_count(&marker), 1);

    // Aborted before it ran.
    let m = marker.clone();
    let op = t.ui_thread.invoke_async_local(move || drop(m));
    assert_eq!(Rc::strong_count(&marker), 2);
    assert!(op.abort());
    assert_eq!(Rc::strong_count(&marker), 1);

    // Cleared without running.
    let m = marker.clone();
    t.ui_thread.post_local(move || drop(m), DispatcherPriority::NORMAL);
    assert_eq!(t.ui_thread.get_jobs().len(), 1);
    t.ui_thread.clear_jobs();
    assert_eq!(Rc::strong_count(&marker), 1);

    assert!(local.jobs.borrow().is_empty());
    assert!(local.results.borrow().is_empty());
}

#[test]
fn waiting_on_the_executing_operation_panics() {
    let t = DispatcherTests::new();
    let slot: Rc<RefCell<Option<DispatcherOperation>>> = Rc::new(RefCell::new(None));
    let s = slot.clone();
    let op = t.ui_thread.invoke_async_local(move || {
        let this = s.borrow().clone().unwrap();
        let _ = this.wait();
    });
    *slot.borrow_mut() = Some(op.clone());
    t.ui_thread.run_jobs(None);

    let result = catch_unwind(AssertUnwindSafe(|| op.wait()));
    assert_eq!(
        message_of(&*result.unwrap_err()),
        Some("A thread cannot wait on operations already running on the same thread.")
    );
}

#[test]
fn dispatcher_operation_is_a_future() {
    let t = DispatcherTests::new();
    let waker = CountingWaker::new();

    let mut op = t.ui_thread.invoke_async(|| 3);
    assert!(poll_once(&mut op, &waker).is_pending());
    assert!(poll_once(&mut op, &waker).is_pending());
    t.ui_thread.run_jobs(None);
    assert_eq!(waker.wakes.load(Ordering::SeqCst), 1);
    assert_eq!(poll_once(&mut op, &waker), Poll::Ready(Ok(3)));

    let mut aborted = t.ui_thread.invoke_async_local(|| Rc::new(1));
    assert!(poll_once(&mut aborted, &waker).is_pending());
    aborted.abort();
    assert_eq!(waker.wakes.load(Ordering::SeqCst), 2);
    assert_eq!(poll_once(&mut aborted, &waker), Poll::Ready(Err(OperationCanceledError)));

    let mut panicking = t.ui_thread.invoke_async(|| panic!("boom"));
    t.ui_thread.run_jobs(None);
    let result = catch_unwind(AssertUnwindSafe(|| poll_once(&mut panicking, &waker)));
    assert_eq!(message_of(&*result.unwrap_err()), Some("boom"));
}

#[test]
fn dispatcher_resume_continues_on_dispatcher_thread() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));
    let dispatcher = Dispatcher::current_dispatcher();
    let token_source = CancellationTokenSource::new();

    let waker = CountingWaker::new();
    let stop = token_source.clone();
    *waker.on_wake.lock().unwrap() = Some(Box::new(move || stop.cancel()));

    // First awaited on another thread...
    let mut awaitable = dispatcher.resume();
    let w = waker.clone();
    let mut awaitable = thread::spawn(move || {
        assert!(poll_once(&mut awaitable, &w).is_pending());
        awaitable
    })
    .join()
    .unwrap();
    assert_eq!(waker.wakes.load(Ordering::SeqCst), 0);

    // ...and resumed from the dispatcher thread.
    dispatcher.main_loop(&token_source.token());
    assert_eq!(waker.wakes.load(Ordering::SeqCst), 1);
    assert_eq!(*waker.woken_on.lock().unwrap(), Some(thread::current().id()));
    assert!(poll_once(&mut awaitable, &waker).is_ready());
}

#[test]
fn dispatcher_yield_continues_on_current_thread() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));
    let dispatcher = Dispatcher::current_dispatcher();

    let order = Rc::new(RefCell::new(Vec::new()));
    let waker = CountingWaker::new();
    let mut awaitable = Dispatcher::yield_now();
    assert!(poll_once(&mut awaitable, &waker).is_pending());
    // Yielding lets work that is already queued at a higher priority run first.
    dispatcher.post_local(add(&order, "normal"), DispatcherPriority::NORMAL);
    assert_eq!(waker.wakes.load(Ordering::SeqCst), 0);

    dispatcher.run_jobs(Some(DispatcherPriority::NORMAL));
    assert_eq!(names(&order), ["normal"]);
    assert!(poll_once(&mut awaitable, &waker).is_pending());

    dispatcher.run_jobs(None);
    assert_eq!(waker.wakes.load(Ordering::SeqCst), 1);
    assert_eq!(*waker.woken_on.lock().unwrap(), Some(thread::current().id()));
    assert!(poll_once(&mut awaitable, &waker).is_ready());
}

#[test]
fn dispatcher_timer_ticks_when_due() {
    let _t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::reset_for_unit_tests();
    let dispatcher = Dispatcher::new(Some(impl_.clone()));
    impl_.set_now(100);

    let ticks = Rc::new(Cell::new(0));
    let timer = DispatcherTimer::with_interval(Duration::from_millis(50), DispatcherPriority::NORMAL, &dispatcher);
    let c = ticks.clone();
    timer.tick(move |_| c.set(c.get() + 1));
    assert!(!timer.is_enabled());
    assert_eq!(DispatcherTimer::active_timers_count(), 0);

    timer.start();
    assert!(timer.is_enabled());
    assert_eq!(DispatcherTimer::active_timers_count(), 1);
    // Queueing the (inactive) operation asks for background processing,
    // which does nothing as long as the operation is not promoted.
    assert_eq!(impl_.next_timer(), Some(101));
    impl_.execute_timer();
    assert!(!impl_.asked_for_signal());
    assert_eq!(impl_.next_timer(), Some(150));
    assert_eq!(Dispatcher::snapshot_timers_for_unit_tests().len(), 1);
    // The operation is queued but inactive.
    assert!(dispatcher.has_jobs_with_priority(DispatcherPriority::INACTIVE));
    assert!(!dispatcher.has_jobs_with_priority(DispatcherPriority::SYSTEM_IDLE));
    dispatcher.run_jobs(None);
    assert_eq!(ticks.get(), 0);

    impl_.execute_timer();
    assert!(impl_.asked_for_signal());
    impl_.execute_signal();
    assert_eq!(ticks.get(), 1);
    // Restarted.
    impl_.execute_timer();
    assert_eq!(impl_.now(), 151);
    assert_eq!(impl_.next_timer(), Some(200));

    timer.set_interval(Duration::from_millis(10));
    assert_eq!(timer.interval(), Duration::from_millis(10));
    assert_eq!(impl_.next_timer(), Some(161));

    timer.stop();
    assert!(!timer.is_enabled());
    assert_eq!(DispatcherTimer::active_timers_count(), 0);
    assert_eq!(impl_.next_timer(), None);
    assert!(!dispatcher.has_jobs_with_priority(DispatcherPriority::INACTIVE));
    assert!(Dispatcher::snapshot_timers_for_unit_tests().is_empty());

    timer.set_is_enabled(true);
    assert_eq!(Dispatcher::snapshot_timers_for_unit_tests()[0].due_time_in_ms(), 161);
    timer.set_is_enabled(false);
    assert_eq!(impl_.next_timer(), Some(152));
    impl_.execute_timer();
    assert_eq!(impl_.next_timer(), None);
    assert_eq!(ticks.get(), 1);
}

#[test]
fn dispatcher_timer_with_zero_interval_is_promoted_immediately() {
    let _t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::reset_for_unit_tests();
    let dispatcher = Dispatcher::new(Some(impl_.clone()));

    let ticks = Rc::new(Cell::new(0));
    let c = ticks.clone();
    let timer = DispatcherTimer::with_callback(Duration::ZERO, DispatcherPriority::NORMAL, move |timer| {
        c.set(c.get() + 1);
        if c.get() == 3 {
            timer.stop();
        }
    });
    assert!(Arc::ptr_eq(timer.dispatcher(), &dispatcher));
    assert!(impl_.asked_for_signal());
    assert!(Dispatcher::snapshot_timers_for_unit_tests().is_empty());
    dispatcher.run_jobs(None);
    assert_eq!(ticks.get(), 3);
    assert!(!timer.is_enabled());
}

#[test]
fn dispatcher_timer_run_and_run_once() {
    let _t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::reset_for_unit_tests();
    let dispatcher = Dispatcher::new(Some(impl_.clone()));

    let runs = Rc::new(Cell::new(0));
    let once = Rc::new(Cell::new(0));
    let r = runs.clone();
    let _run = DispatcherTimer::run(
        move || {
            r.set(r.get() + 1);
            r.get() < 2
        },
        Duration::from_millis(10),
        DispatcherPriority::NORMAL,
    );
    let o = once.clone();
    let _once = DispatcherTimer::run_once(move || o.set(o.get() + 1), Duration::from_millis(25), DispatcherPriority::NORMAL);
    let c = once.clone();
    let canceled =
        DispatcherTimer::run_once(move || c.set(c.get() + 100), Duration::from_millis(5), DispatcherPriority::NORMAL);
    canceled.dispose();

    for _ in 0..5 {
        impl_.execute_timer();
        impl_.execute_signal();
        dispatcher.run_jobs(None);
    }

    assert_eq!(runs.get(), 2);
    assert_eq!(once.get(), 1);
    assert_eq!(impl_.next_timer(), None);
    assert_eq!(DispatcherTimer::active_timers_count(), 0);
}

#[test]
fn timers_keep_their_due_time_when_the_implementation_changes() {
    let t = DispatcherTests::new();
    // Started against the pre-initialization implementation's clock.
    let ticks = Rc::new(Cell::new(0));
    let c = ticks.clone();
    let _timer = DispatcherTimer::with_callback(Duration::from_millis(500), DispatcherPriority::NORMAL, move |timer| {
        c.set(c.get() + 1);
        timer.stop();
    });

    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    impl_.set_now(1_000_000);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());

    // The due time moved to the new clock (allowing for the time the test took).
    assert_eq!(t.ui_thread.now(), 1_000_000);
    let due = Dispatcher::snapshot_timers_for_unit_tests()[0].due_time_in_ms();
    assert!(due > 1_000_400 && due <= 1_000_500, "due = {due}");

    // First the background processing request made for the inactive operation...
    impl_.execute_timer();
    assert_eq!(ticks.get(), 0);
    // ...then the timer itself.
    assert_eq!(impl_.next_timer(), Some(due));
    impl_.execute_timer();
    impl_.execute_signal();
    assert_eq!(ticks.get(), 1);
}

#[test]
fn shutdown_events_are_raised_once_each() {
    let t = DispatcherTests::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let e = events.clone();
    let d = t.ui_thread.clone();
    t.ui_thread.shutdown_started(move |dispatcher| {
        assert!(Arc::ptr_eq(dispatcher, &d));
        assert!(!dispatcher.has_shutdown_started());
        e.borrow_mut().push("started".to_string());
        // Reentrancy is ignored.
        dispatcher.invoke_shutdown();
    });
    t.ui_thread.shutdown_finished(add_arc(&events, "finished"));
    let pending = t.ui_thread.invoke_async_local(|| ());

    t.ui_thread.invoke_shutdown();
    assert_eq!(names(&events), ["started", "finished"]);
    assert_eq!(pending.status(), DispatcherOperationStatus::Aborted);

    // Nothing can be queued anymore.
    let late = t.ui_thread.invoke_async(|| ());
    assert_eq!(late.status(), DispatcherOperationStatus::Aborted);
    assert_eq!(t.ui_thread.invoke_local_with_priority(|| 1, DispatcherPriority::NORMAL), Err(OperationCanceledError));
    t.ui_thread.invoke_shutdown();
    assert_eq!(names(&events), ["started", "finished"]);
}

fn add_arc(events: &Rc<RefCell<Vec<String>>>, name: &'static str) -> impl Fn(&Arc<Dispatcher>) + 'static {
    let events = events.clone();
    move |_| events.borrow_mut().push(name.to_string())
}

#[test]
fn operations_queued_on_an_exited_thread_are_aborted() {
    let (dispatcher, op) = thread::spawn(|| {
        let dispatcher = Dispatcher::current_dispatcher();
        let op = dispatcher.invoke_async(|| 1);
        (dispatcher, op)
    })
    .join()
    .unwrap();

    // The thread is gone: waiting must not block forever.
    assert_eq!(op.status(), DispatcherOperationStatus::Aborted);
    assert_eq!(op.result(), Err(OperationCanceledError));
    assert_eq!(dispatcher.invoke_async(|| 2).result(), Err(OperationCanceledError));
    assert!(Dispatcher::from_thread(dispatcher.thread()).is_some());
}

#[test]
fn initializing_twice_panics() {
    let _t = DispatcherTests::new();
    Dispatcher::initialize_ui_thread_dispatcher(SimpleDispatcherImpl::new(SimpleKind::Simple));
    let result = catch_unwind(|| {
        Dispatcher::initialize_ui_thread_dispatcher(SimpleDispatcherImpl::new(SimpleKind::Simple));
    });
    assert_eq!(message_of(&*result.unwrap_err()), Some("UI thread dispatcher is already initialized"));
}

#[test]
fn implementation_of_another_thread_is_rejected() {
    let _t = DispatcherTests::new();
    let foreign_thread = thread::spawn(|| thread::current().id()).join().unwrap();
    let foreign = SimpleDispatcherImpl::new_on_thread(SimpleKind::Simple, foreign_thread);
    let result = catch_unwind(AssertUnwindSafe(|| Dispatcher::initialize_ui_thread_dispatcher(foreign)));
    assert_eq!(message_of(&*result.unwrap_err()), Some("IDispatcherImpl belongs to a different thread"));
}

#[test]
fn push_frame_requires_a_controlled_implementation() {
    let t = DispatcherTests::new();
    Dispatcher::initialize_ui_thread_dispatcher(SimpleDispatcherImpl::new(SimpleKind::WithBackgroundProcessing));
    assert!(!t.ui_thread.supports_run_loops());
    let d = t.ui_thread.clone();
    let result = catch_unwind(AssertUnwindSafe(|| d.main_loop(&CancellationToken::none())));
    assert!(message_of(&*result.unwrap_err()).unwrap().starts_with("Operation is not supported on this platform"));
    let d = t.ui_thread.clone();
    let result = catch_unwind(AssertUnwindSafe(|| d.push_frame(&DispatcherFrame::new())));
    assert!(message_of(&*result.unwrap_err()).unwrap().starts_with("Operation is not supported on this platform"));
}

#[test]
fn handles_are_thread_safe() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Dispatcher>();
    assert_send_sync::<DispatcherFrame>();
    assert_send_sync::<DispatcherOperation<i32>>();
    assert_send_sync::<CancellationToken>();
    assert_send_sync::<CancellationTokenSource>();
    assert_send_sync::<DispatcherTask<i32>>();
    assert_send_sync::<FerroSynchronizationContext>();
    assert_send_sync::<DispatcherTaskScheduler>();
}

#[test]
fn ui_thread_is_the_first_dispatcher_outside_of_unit_test_scopes() {
    // Runs on a thread without a unit-test scope, so it exercises the
    // process-wide UI thread slot. Every other test is isolated from it.
    thread::spawn(|| {
        let ui_thread = Dispatcher::ui_thread();
        assert!(Arc::ptr_eq(&ui_thread, &Dispatcher::ui_thread()));
        assert!(Arc::ptr_eq(&ui_thread, &Dispatcher::try_get_ui_thread().unwrap()));

        let current = Dispatcher::current_dispatcher();
        let other = thread::spawn(|| (Dispatcher::ui_thread(), Dispatcher::current_dispatcher())).join().unwrap();
        assert!(Arc::ptr_eq(&other.0, &ui_thread));
        assert!(!Arc::ptr_eq(&other.1, &current));

        if Arc::ptr_eq(&ui_thread, &current) {
            // Do not leave the dispatcher of this short-lived thread behind
            // as the process-wide one.
            Dispatcher::reset_for_unit_tests();
            assert!(Dispatcher::from_thread(thread::current().id()).is_none());
        }
    })
    .join()
    .unwrap();
}

// ---------------------------------------------------------------------------
// Synchronization context, futures on the dispatcher and timeouts.
// ---------------------------------------------------------------------------

/// A future that is completed by hand, from any thread.
struct Oneshot<T> {
    shared: Arc<Mutex<(Option<T>, Option<Waker>)>>,
}

struct OneshotSender<T> {
    shared: Arc<Mutex<(Option<T>, Option<Waker>)>>,
}

fn oneshot<T>() -> (OneshotSender<T>, Oneshot<T>) {
    let shared = Arc::new(Mutex::new((None, None)));
    (OneshotSender { shared: shared.clone() }, Oneshot { shared })
}

impl<T> OneshotSender<T> {
    fn send(&self, value: T) {
        let waker = {
            let mut shared = self.shared.lock().unwrap();
            shared.0 = Some(value);
            shared.1.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<T> Future for Oneshot<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut shared = self.shared.lock().unwrap();
        match shared.0.take() {
            Some(value) => Poll::Ready(value),
            None => {
                shared.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Completes after `duration` of dispatcher time; the stand-in for a timer task.
fn delay(duration: Duration) -> Oneshot<()> {
    let (sender, receiver) = oneshot();
    DispatcherTimer::run_once(move || sender.send(()), duration, DispatcherPriority::NORMAL);
    receiver
}

/// Completes on another thread and yields that thread's id; the stand-in for
/// work that continues on a pool thread.
fn workload_on_another_thread() -> Oneshot<ThreadId> {
    let (sender, receiver) = oneshot();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(1));
        sender.send(thread::current().id());
    });
    receiver
}

fn current_priority() -> DispatcherPriority {
    FerroSynchronizationContext::current().expect("a current context").priority()
}

#[test]
fn dispatcher_operations_have_context_with_proper_priority() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));
    FerroSynchronizationContext::set_current(None);
    let disp = Dispatcher::ui_thread();
    let priorities = Rc::new(RefCell::new(Vec::new()));

    let dump_current_priority = {
        let priorities = priorities.clone();
        move || priorities.borrow_mut().push(current_priority())
    };

    disp.post_local(dump_current_priority.clone(), DispatcherPriority::NORMAL);
    disp.post_local(dump_current_priority.clone(), DispatcherPriority::LOADED);
    disp.post_local(dump_current_priority.clone(), DispatcherPriority::INPUT);
    {
        let dump = dump_current_priority.clone();
        let d = disp.clone();
        disp.post_local(
            move || {
                dump();
                d.exit_all_frames();
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    disp.main_loop(&CancellationToken::none());

    disp.send_local(dump_current_priority.clone(), Some(DispatcherPriority::SEND));
    disp.invoke_local_with_priority(dump_current_priority.clone(), DispatcherPriority::SEND).unwrap();
    let dump = dump_current_priority.clone();
    let one = disp
        .invoke_local_with_priority(
            move || {
                dump();
                1
            },
            DispatcherPriority::SEND,
        )
        .unwrap();
    assert_eq!(one, 1);

    assert_eq!(
        *priorities.borrow(),
        [
            DispatcherPriority::NORMAL,
            DispatcherPriority::LOADED,
            DispatcherPriority::INPUT,
            DispatcherPriority::BACKGROUND,
            DispatcherPriority::SEND,
            DispatcherPriority::SEND,
            DispatcherPriority::SEND,
        ]
    );
    // The context is restored after each of them.
    assert!(FerroSynchronizationContext::current().is_none());
}

#[test]
fn dispatcher_invoke_async_unwraps_tasks() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(Rc::new(ManagedDispatcherImpl::new(None)));
    let async_method_stage = Rc::new(Cell::new(0));

    let async_method = {
        let stage = async_method_stage.clone();
        move || {
            let stage = stage.clone();
            async move {
                stage.set(1);
                delay(Duration::from_millis(40)).await;
                stage.set(2);
            }
        }
    };

    let async_method_with_result = || async {
        delay(Duration::from_millis(20)).await;
        1
    };

    let test = {
        let stage = async_method_stage.clone();
        async move {
            let ui_thread = Dispatcher::ui_thread();
            ui_thread.invoke_async_task_local(async_method.clone()).await.unwrap();
            assert_eq!(stage.get(), 2);
            assert_eq!(ui_thread.invoke_async_task(async_method_with_result).await, Ok(1));
            stage.set(0);

            ui_thread
                .invoke_async_task_local_with_priority(async_method.clone(), DispatcherPriority::DEFAULT)
                .await
                .unwrap();
            assert_eq!(stage.get(), 2);
            assert_eq!(
                ui_thread.invoke_async_task_with_priority(async_method_with_result, DispatcherPriority::DEFAULT).await,
                Ok(1)
            );

            Dispatcher::ui_thread().exit_all_frames();
        }
    };

    let t = Dispatcher::ui_thread().to_task_scheduler().spawn_local(test);
    assert!(!t.is_completed());
    let cts = cancel_after(Duration::from_secs(3));
    Dispatcher::ui_thread().main_loop(&cts.token());
    assert!(t.is_completed_successfully());
    t.result().unwrap();
}

#[test]
fn dispatcher_resume_continues_on_current_thread() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));

    let token_source = CancellationTokenSource::new();
    let dispatcher = Dispatcher::current_dispatcher();

    let d = dispatcher.clone();
    let stop = token_source.clone();
    let workload = dispatcher.invoke_async_task_local(move || async move {
        assert!(d.check_access());
        assert_eq!(current_priority(), DispatcherPriority::DEFAULT);

        let other_thread = workload_on_another_thread().await;
        assert_ne!(other_thread, thread::current().id());

        d.resume().await;
        assert!(d.check_access());
        // The continuation runs inside the job of the requested priority.
        assert_eq!(current_priority(), DispatcherPriority::BACKGROUND);

        d.resume_with_priority(DispatcherPriority::RENDER).await;
        assert_eq!(current_priority(), DispatcherPriority::RENDER);

        stop.cancel();
    });

    let timeout = cancel_after(Duration::from_secs(5));
    let stop = token_source.clone();
    timeout.token().register(move || stop.cancel());
    dispatcher.main_loop(&token_source.token());
    assert!(workload.is_completed_successfully());
}

#[test]
fn dispatcher_yield_continues_on_current_thread_in_a_task() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));

    let token_source = CancellationTokenSource::new();
    let dispatcher = Dispatcher::current_dispatcher();
    let order = Rc::new(RefCell::new(Vec::new()));

    let d = dispatcher.clone();
    let stop = token_source.clone();
    let o = order.clone();
    let workload = dispatcher.invoke_async_task_local(move || async move {
        assert!(d.check_access());
        // Queued behind the yield in arrival order, but with a higher priority.
        d.post_local(add(&o, "posted"), DispatcherPriority::NORMAL);

        Dispatcher::yield_now().await;
        assert!(d.check_access());
        o.borrow_mut().push("resumed".to_string());
        assert_eq!(current_priority(), DispatcherPriority::BACKGROUND);

        stop.cancel();
    });

    dispatcher.main_loop(&token_source.token());
    assert!(workload.is_completed_successfully());
    assert_eq!(names(&order), ["posted", "resumed"]);
}

#[test]
fn await_with_priority_runs_on_current_thread() {
    let _t = DispatcherTests::new();
    let _services = DispatcherServices::new(SimpleDispatcherImpl::new(SimpleKind::Controlled));

    let token_source = CancellationTokenSource::new();
    let dispatcher = Dispatcher::current_dispatcher();

    let d = dispatcher.clone();
    let stop = token_source.clone();
    let workload = dispatcher.invoke_async_task_local_with_priority(
        move || async move {
            assert!(d.check_access());
            assert_eq!(current_priority(), DispatcherPriority::BACKGROUND);
            let task_without_result = async {
                workload_on_another_thread().await;
            };

            d.await_with_priority(task_without_result, DispatcherPriority::DEFAULT).await;

            assert!(d.check_access());
            assert_eq!(current_priority(), DispatcherPriority::DEFAULT);
            let task_with_result = workload_on_another_thread();

            let thread_id = d.await_with_priority(task_with_result, DispatcherPriority::LOADED).await;

            assert!(d.check_access());
            assert_ne!(thread_id, thread::current().id());
            assert_eq!(current_priority(), DispatcherPriority::LOADED);

            // An already completed task still resumes through the dispatcher.
            let ran_inline = Rc::new(Cell::new(true));
            let r = ran_inline.clone();
            d.post_local(move || r.set(false), DispatcherPriority::SEND);
            assert_eq!(d.await_with_priority(std::future::ready(5), DispatcherPriority::DEFAULT).await, 5);
            assert!(!ran_inline.get());

            stop.cancel();
            Rc::new(thread_id)
        },
        DispatcherPriority::BACKGROUND,
    );

    let timeout = cancel_after(Duration::from_secs(5));
    let stop = token_source.clone();
    timeout.token().register(move || stop.cancel());
    dispatcher.main_loop(&token_source.token());
    assert!(workload.is_completed_successfully());
    // A result that is not `Send` is read on the dispatcher thread.
    assert_ne!(*workload.result().unwrap(), thread::current().id());
}

#[test]
fn dispatcher_can_act_as_task_scheduler() {
    let _t = DispatcherTests::new();
    let impl_ = SimpleDispatcherImpl::new(SimpleKind::Simple);
    Dispatcher::initialize_ui_thread_dispatcher(impl_.clone());
    let scheduler = Dispatcher::ui_thread().to_task_scheduler();
    assert_eq!(scheduler.priority(), DispatcherPriority::DEFAULT);

    // Scheduled from another thread, as a continuation of work done there.
    let continuation = thread::spawn(move || scheduler.spawn(async { thread::current().id() })).join().unwrap();
    assert!(impl_.asked_for_signal());
    assert!(!continuation.is_completed());
    impl_.execute_signal();
    assert_eq!(continuation.result(), Ok(Dispatcher::ui_thread().thread()));
}

#[test]
fn task_scheduler_takes_the_priority_of_the_current_context() {
    let t = DispatcherTests::new();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let d = t.ui_thread.clone();
    let s = seen.clone();
    t.ui_thread.post_local(
        move || {
            let scheduler = d.to_task_scheduler();
            s.borrow_mut().push(scheduler.priority());
            let s = s.clone();
            // Queued, not run inline.
            let task = scheduler.spawn_local(async move {
                s.borrow_mut().push(current_priority());
                Rc::new(1)
            });
            assert!(!task.is_completed());
        },
        DispatcherPriority::RENDER,
    );
    t.ui_thread.run_jobs(None);
    assert_eq!(*seen.borrow(), [DispatcherPriority::RENDER, DispatcherPriority::RENDER]);

    assert_eq!(
        t.ui_thread.to_task_scheduler_with_priority(DispatcherPriority::INPUT).priority(),
        DispatcherPriority::INPUT
    );
    assert!(Arc::ptr_eq(t.ui_thread.to_task_scheduler().dispatcher(), &t.ui_thread));
}

#[test]
fn synchronization_context_ensure_and_install() {
    let t = DispatcherTests::new();
    assert!(FerroSynchronizationContext::current().is_none());

    // The per-priority contexts are cached.
    let normal = t.ui_thread.get_context_with_priority(DispatcherPriority::NORMAL);
    assert!(Arc::ptr_eq(&normal, &t.ui_thread.get_context_with_priority(DispatcherPriority::NORMAL)));
    assert!(Arc::ptr_eq(normal.dispatcher(), &t.ui_thread));

    {
        let restore = FerroSynchronizationContext::ensure(DispatcherPriority::INPUT);
        assert_eq!(current_priority(), DispatcherPriority::INPUT);
        {
            // Same priority: nothing to change, nothing to restore.
            let _same = FerroSynchronizationContext::ensure(DispatcherPriority::INPUT);
            let _nested = FerroSynchronizationContext::ensure_with_dispatcher(&t.ui_thread, DispatcherPriority::RENDER);
            assert_eq!(current_priority(), DispatcherPriority::RENDER);
        }
        assert_eq!(current_priority(), DispatcherPriority::INPUT);
        restore.dispose();
        assert!(FerroSynchronizationContext::current().is_none());
        // Disposing twice (and dropping afterwards) changes nothing.
        FerroSynchronizationContext::set_current(Some(normal.clone()));
        restore.dispose();
    }
    assert!(Arc::ptr_eq(&FerroSynchronizationContext::current().unwrap(), &normal));
    FerroSynchronizationContext::set_current(None);

    assert!(FerroSynchronizationContext::auto_install());
    FerroSynchronizationContext::install_if_needed();
    assert!(Arc::ptr_eq(&FerroSynchronizationContext::current().unwrap(), &normal));
    let own = FerroSynchronizationContext::with_priority(DispatcherPriority::LOADED);
    FerroSynchronizationContext::set_current(Some(own.clone()));
    FerroSynchronizationContext::install_if_needed();
    assert!(Arc::ptr_eq(&FerroSynchronizationContext::current().unwrap(), &own));
    assert_eq!(FerroSynchronizationContext::new().priority(), DispatcherPriority::DEFAULT);
    FerroSynchronizationContext::set_current(None);
}

#[test]
fn synchronization_context_post_and_send() {
    let t = DispatcherTests::new();
    let context = t.ui_thread.get_context_with_priority(DispatcherPriority::BACKGROUND);
    let seen = Rc::new(RefCell::new(Vec::new()));

    let s = seen.clone();
    context.post_local(move || s.borrow_mut().push(("post", current_priority())));
    let s = seen.clone();
    // Same thread: runs right away, at send priority.
    context.send_local(move || s.borrow_mut().push(("send", current_priority())));
    let on_thread = Arc::new(AtomicBool::new(false));
    let o = on_thread.clone();
    context.send(move || o.store(true, Ordering::SeqCst));
    assert!(on_thread.load(Ordering::SeqCst));
    assert_eq!(*seen.borrow(), [("send", DispatcherPriority::SEND)]);
    t.ui_thread.run_jobs(None);
    assert_eq!(*seen.borrow(), [("send", DispatcherPriority::SEND), ("post", DispatcherPriority::BACKGROUND)]);

    // From another thread `send` queues at the context's priority and waits.
    let ui_thread_id = thread::current().id();
    let done = CancellationTokenSource::new();
    let finish = done.clone();
    let c = context.clone();
    let worker = thread::spawn(move || {
        let ran = Arc::new(Mutex::new(None));
        let r = ran.clone();
        c.send(move || *r.lock().unwrap() = Some((thread::current().id(), current_priority())));
        let r = ran.clone();
        c.post(move || {
            r.lock().unwrap().take();
            finish.cancel();
        });
        let seen = *ran.lock().unwrap();
        seen
    });
    let timeout = cancel_after(Duration::from_secs(10));
    let stop = timeout.clone();
    done.token().register(move || stop.cancel());
    t.ui_thread.main_loop(&timeout.token());
    assert_eq!(worker.join().unwrap(), Some((ui_thread_id, DispatcherPriority::BACKGROUND)));
}

#[test]
fn send_reports_panics_to_the_unhandled_exception_handlers() {
    let t = DispatcherTests::new();
    let handled = Rc::new(Cell::new(0));
    let h = handled.clone();
    t.ui_thread.unhandled_exception(move |args| {
        h.set(h.get() + 1);
        args.set_handled(true);
    });
    let context = t.ui_thread.get_context_with_priority(DispatcherPriority::NORMAL);
    context.send_local(|| panic!("inline"));
    assert_eq!(handled.get(), 1);
    t.ui_thread.send_local(|| panic!("queued"), Some(DispatcherPriority::NORMAL));
    assert_eq!(handled.get(), 2);
}

#[test]
fn disable_processing_keeps_the_current_context() {
    // The context half of the reference test: there the helper that replaces
    // the pumping wait of the runtime is swapped in unless a dispatcher
    // context is current. Blocking never pumps here, so all that is left to
    // check is that the current context is left alone.
    let t = DispatcherTests::new();
    assert!(FerroSynchronizationContext::current().is_none());
    let disabled = t.ui_thread.disable_processing();
    assert!(FerroSynchronizationContext::current().is_none());
    disabled.dispose();

    let context = FerroSynchronizationContext::with_dispatcher(&t.ui_thread, DispatcherPriority::DEFAULT);
    FerroSynchronizationContext::set_current(Some(context.clone()));
    let disabled = t.ui_thread.disable_processing();
    assert!(Arc::ptr_eq(&FerroSynchronizationContext::current().unwrap(), &context));
    disabled.dispose();
    assert!(Arc::ptr_eq(&FerroSynchronizationContext::current().unwrap(), &context));
    FerroSynchronizationContext::set_current(None);
}

#[test]
fn invoke_async_task_outcomes() {
    let t = DispatcherTests::new();

    // The callback panics before producing a future.
    let from_callback = t.ui_thread.invoke_async_task(|| -> std::future::Ready<i32> { panic!("callback") });
    // The future panics.
    let from_future = t.ui_thread.invoke_async_task(|| async {
        if true {
            panic!("future");
        }
        1
    });
    // The first poll happens inside the operation that ran the callback.
    let order = Rc::new(RefCell::new(Vec::new()));
    let o = order.clone();
    let d = t.ui_thread.clone();
    let stuck = t.ui_thread.invoke_async_task_local(move || {
        o.borrow_mut().push("callback".to_string());
        d.post_local(add(&o, "posted"), DispatcherPriority::SEND);
        async move {
            o.borrow_mut().push("first poll".to_string());
            std::future::pending::<()>().await;
        }
    });
    assert!(!from_callback.is_completed());

    t.ui_thread.run_jobs(None);
    assert_eq!(names(&order), ["callback", "first poll", "posted"]);

    assert!(from_callback.is_faulted());
    let c = from_callback.clone();
    let result = catch_unwind(AssertUnwindSafe(|| c.wait()));
    assert_eq!(message_of(&*result.unwrap_err()), Some("callback"));
    assert!(from_future.is_faulted());
    let result = catch_unwind(AssertUnwindSafe(|| from_future.result()));
    assert_eq!(message_of(&*result.unwrap_err()), Some("future"));

    // Waiting for a running task on the dispatcher thread cannot work.
    assert!(!stuck.is_completed());
    let s = stuck.clone();
    let result = catch_unwind(AssertUnwindSafe(|| s.wait()));
    assert_eq!(message_of(&*result.unwrap_err()), Some("Synchronous wait is only supported on non-UI threads"));

    // Shutting the dispatcher down cancels what is still running...
    t.ui_thread.invoke_shutdown();
    assert!(stuck.is_canceled());
    assert_eq!(stuck.wait(), Err(OperationCanceledError));
    // ...and what can no longer start.
    let late = t.ui_thread.invoke_async_task(|| async { 1 });
    assert!(late.is_canceled());
    assert_eq!(late.result(), Err(OperationCanceledError));
    assert!(t.ui_thread.try_local().unwrap().tasks.borrow().is_empty());
}

#[test]
fn invoke_async_task_from_another_thread() {
    let t = DispatcherTests::new();
    let dispatcher = t.ui_thread.clone();
    let ui_thread_id = thread::current().id();
    let done = CancellationTokenSource::new();
    let finish = done.clone();
    let worker = thread::spawn(move || {
        let task = dispatcher.invoke_async_task(|| async {
            // Not `Send`: the future lives on the dispatcher thread.
            let on_thread = Rc::new(thread::current().id());
            delay(Duration::from_millis(5)).await;
            Dispatcher::yield_now().await;
            (*on_thread, thread::current().id())
        });
        let result = task.result();
        finish.cancel();
        result
    });

    let timeout = cancel_after(Duration::from_secs(10));
    let stop = timeout.clone();
    done.token().register(move || stop.cancel());
    t.ui_thread.main_loop(&timeout.token());
    assert_eq!(worker.join().unwrap(), Ok((ui_thread_id, ui_thread_id)));
}

#[test]
fn dispatcher_task_is_a_future_for_other_executors() {
    let t = DispatcherTests::new();
    let waker = CountingWaker::new();
    let (sender, receiver) = oneshot();
    let mut task = t.ui_thread.invoke_async_task(move || receiver);
    assert!(poll_once(&mut task, &waker).is_pending());
    t.ui_thread.run_jobs(None);
    assert!(poll_once(&mut task, &waker).is_pending());
    assert_eq!(waker.wakes.load(Ordering::SeqCst), 0);

    sender.send(11);
    // Completion is delivered by a dispatcher job.
    assert!(poll_once(&mut task, &waker).is_pending());
    t.ui_thread.run_jobs(None);
    assert_eq!(waker.wakes.load(Ordering::SeqCst), 1);
    assert_eq!(poll_once(&mut task, &waker), Poll::Ready(Ok(11)));
}

#[test]
fn invoke_with_timeout_from_another_thread() {
    let t = DispatcherTests::new();
    let dispatcher = t.ui_thread.clone();
    let ran = Arc::new(AtomicBool::new(false));
    let r = ran.clone();
    // The dispatcher thread is not processing its queue.
    let result = thread::spawn(move || {
        dispatcher.invoke_with_timeout(
            move || r.store(true, Ordering::SeqCst),
            DispatcherPriority::NORMAL,
            &CancellationToken::none(),
            Duration::from_millis(30),
        )
    })
    .join()
    .unwrap();
    assert_eq!(result, Err(DispatcherInvokeError::Timeout));
    // The operation was aborted: it does not run later either.
    t.ui_thread.run_jobs(None);
    assert!(!ran.load(Ordering::SeqCst));

    // Within the timeout.
    let dispatcher = t.ui_thread.clone();
    let done = CancellationTokenSource::new();
    let finish = done.clone();
    let worker = thread::spawn(move || {
        let result = dispatcher.invoke_with_timeout(
            || 7,
            DispatcherPriority::NORMAL,
            &CancellationToken::none(),
            Duration::from_secs(10),
        );
        finish.cancel();
        result
    });
    let timeout = cancel_after(Duration::from_secs(10));
    let stop = timeout.clone();
    done.token().register(move || stop.cancel());
    t.ui_thread.main_loop(&timeout.token());
    assert_eq!(worker.join().unwrap(), Ok(7));
}

#[test]
fn invoke_with_timeout_on_the_dispatcher_thread() {
    // The pre-initialization implementation supports run loops and has a real clock.
    let t = DispatcherTests::new();
    assert!(t.ui_thread.supports_run_loops());
    let none = CancellationToken::none();

    // Send priority runs inline, whatever the timeout.
    assert_eq!(t.ui_thread.invoke_with_timeout(|| 1, DispatcherPriority::SEND, &none, Duration::ZERO), Ok(1));
    // Foreground and background priorities pump the queue.
    assert_eq!(
        t.ui_thread.invoke_local_with_timeout(|| Rc::new(2), DispatcherPriority::NORMAL, &none, Duration::from_secs(5))
            .map(|value| *value),
        Ok(2)
    );
    assert_eq!(
        t.ui_thread.invoke_with_timeout(|| 3, DispatcherPriority::BACKGROUND, &none, Duration::from_secs(5)),
        Ok(3)
    );

    // An inactive operation never starts: the nested frame is left by the
    // dispatcher timer and the operation is aborted.
    let started = Instant::now();
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    let result = t.ui_thread.invoke_local_with_timeout(
        move || r.set(true),
        DispatcherPriority::INACTIVE,
        &none,
        Duration::from_millis(30),
    );
    assert_eq!(result, Err(DispatcherInvokeError::Timeout));
    assert!(started.elapsed() >= Duration::from_millis(25));
    assert!(!ran.get());
    assert!(!t.ui_thread.has_jobs_with_priority(DispatcherPriority::INACTIVE));
    assert_eq!(DispatcherTimer::active_timers_count(), 0);

    // A zero timeout gives up at once.
    let result = t.ui_thread.invoke_with_timeout(|| (), DispatcherPriority::INACTIVE, &none, Duration::ZERO);
    assert_eq!(result, Err(DispatcherInvokeError::Timeout));

    // Cancellation is reported as such.
    let canceled = CancellationTokenSource::new();
    canceled.cancel();
    let result = t.ui_thread.invoke_with_timeout(|| (), DispatcherPriority::NORMAL, &canceled.token(), Duration::ZERO);
    assert_eq!(result, Err(DispatcherInvokeError::Canceled));
}

#[test]
fn invoke_with_zero_timeout_without_run_loops() {
    let t = DispatcherTests::new();
    Dispatcher::initialize_ui_thread_dispatcher(SimpleDispatcherImpl::new(SimpleKind::Simple));
    assert!(!t.ui_thread.supports_run_loops());
    let none = CancellationToken::none();
    assert_eq!(
        t.ui_thread.invoke_with_timeout(|| (), DispatcherPriority::INACTIVE, &none, Duration::ZERO),
        Err(DispatcherInvokeError::Timeout)
    );
    // Without run loops everything active is run by hand, so the operation
    // completes before any clock is consulted twice.
    assert_eq!(
        t.ui_thread.invoke_with_timeout(|| 4, DispatcherPriority::BACKGROUND, &none, Duration::from_secs(1)),
        Ok(4)
    );
}

#[test]
fn operation_wait_with_timeout() {
    let t = DispatcherTests::new();

    // Another thread: blocks for at most the timeout.
    let op = t.ui_thread.invoke_async(|| 9);
    let o = op.clone();
    let timed_out = thread::spawn(move || o.wait_with_timeout(Duration::from_millis(30))).join().unwrap();
    assert_eq!(timed_out, Ok(false));
    assert_eq!(op.status(), DispatcherOperationStatus::Pending);
    assert_eq!(op.wait_with_timeout(Duration::ZERO), Ok(false));

    // The dispatcher thread: pumps.
    assert_eq!(op.wait_with_timeout(Duration::from_secs(5)), Ok(true));
    assert_eq!(op.result(), Ok(9));
    let o = op.clone();
    assert_eq!(thread::spawn(move || o.wait_with_timeout(Duration::from_millis(1))).join().unwrap(), Ok(true));

    // A nested frame is left when the time is up; the operation stays queued.
    let inactive = t.ui_thread.invoke_async_with_priority(|| (), DispatcherPriority::INACTIVE);
    let started = Instant::now();
    assert_eq!(inactive.wait_with_timeout(Duration::from_millis(30)), Ok(false));
    assert!(started.elapsed() >= Duration::from_millis(25));
    assert_eq!(inactive.status(), DispatcherOperationStatus::Pending);
    assert_eq!(DispatcherTimer::active_timers_count(), 0);

    inactive.abort();
    assert_eq!(inactive.wait_with_timeout(Duration::from_secs(1)), Err(OperationCanceledError));

    let panicking = t.ui_thread.invoke_async(|| panic!("boom"));
    let result = catch_unwind(AssertUnwindSafe(|| panicking.wait_with_timeout(Duration::from_secs(1))));
    assert_eq!(message_of(&*result.unwrap_err()), Some("boom"));
}

// The execution-context tests of the reference suite check that an operation
// runs under the live culture of the dispatcher thread rather than under the
// one captured when it was queued, and that ambient async-local state flows
// into an operation but not between operations. There is no ambient
// execution context here: thread state is plain thread-local state, which is
// never captured. What remains of those tests is below; the two async-local
// tests have nothing to test.

thread_local! {
    static CULTURE: RefCell<&'static str> = const { RefCell::new("default") };
}

fn culture() -> &'static str {
    CULTURE.with(|culture| *culture.borrow())
}

fn set_culture(value: &'static str) {
    CULTURE.with(|culture| *culture.borrow_mut() = value);
}

#[test]
fn dispatcher_operation_runs_under_live_ui_thread_state_and_does_not_reset_it() {
    let t = DispatcherTests::new();
    let early_op_saw = Rc::new(Cell::new(""));
    let late_op_saw = Rc::new(Cell::new(""));

    // Queued BEFORE the state is set.
    let e = early_op_saw.clone();
    t.ui_thread.invoke_async_local_with_priority(move || e.set(culture()), DispatcherPriority::NORMAL);

    set_culture("custom");

    let l = late_op_saw.clone();
    t.ui_thread.invoke_async_local_with_priority(move || l.set(culture()), DispatcherPriority::NORMAL);

    t.ui_thread.run_jobs(Some(DispatcherPriority::BACKGROUND));

    assert_eq!(early_op_saw.get(), "custom");
    assert_eq!(late_op_saw.get(), "custom");
    assert_eq!(culture(), "custom");
}

#[test]
fn state_set_inside_dispatcher_operation_persists_to_ui_thread() {
    let t = DispatcherTests::new();
    let next_op_saw = Rc::new(Cell::new(""));

    t.ui_thread.invoke_async_with_priority(|| set_culture("custom"), DispatcherPriority::NORMAL);
    let n = next_op_saw.clone();
    t.ui_thread.invoke_async_local_with_priority(move || n.set(culture()), DispatcherPriority::NORMAL);

    t.ui_thread.run_jobs(Some(DispatcherPriority::BACKGROUND));

    assert_eq!(next_op_saw.get(), "custom");
    assert_eq!(culture(), "custom");
}

#[test]
fn dispatcher_operations_use_live_ui_thread_state_not_calling_thread_state() {
    let t = DispatcherTests::new();
    set_culture("ui-thread");

    let frame = DispatcherFrame::new();
    let dispatcher = t.ui_thread.clone();
    let f = frame.clone();
    let calling_thread = thread::spawn(move || {
        // A DIFFERENT state on this non-UI (calling) thread.
        set_culture("call-site");

        let invoke_saw = dispatcher.invoke(culture).unwrap();
        let invoke_async_saw = dispatcher.invoke_async_with_priority(culture, DispatcherPriority::NORMAL);
        dispatcher.invoke_async_with_priority(move || f.set_continue(false), DispatcherPriority::NORMAL);
        (culture(), invoke_saw, invoke_async_saw.result().unwrap())
    });

    t.ui_thread.push_frame(&frame);
    assert_eq!(calling_thread.join().unwrap(), ("call-site", "ui-thread", "ui-thread"));
}

#[test]
fn legacy_platform_threading_interface_is_adapted() {
    use crate::platform::{IPlatformThreadingInterface, PlatformTimerHandle};
    use crate::threading::IPlatformThreadingSignal;
    use crate::reactive::Disposable;

    #[derive(Default)]
    struct Shared {
        signals: Mutex<Vec<DispatcherPriority>>,
    }

    impl IPlatformThreadingSignal for Shared {
        fn signal(&self, priority: DispatcherPriority) {
            self.signals.lock().unwrap().push(priority);
        }
    }

    type Tick = Rc<dyn Fn()>;

    struct Legacy {
        shared: Arc<Shared>,
        signaled: DispatcherImplEvent<Option<DispatcherPriority>>,
        timers: Rc<RefCell<Vec<(Duration, Tick)>>>,
    }

    impl IPlatformThreadingInterface for Legacy {
        fn start_timer(&self, _priority: DispatcherPriority, interval: Duration, tick: Tick) -> PlatformTimerHandle {
            self.timers.borrow_mut().push((interval, tick));
            let timers = self.timers.clone();
            Disposable::create(move || timers.borrow_mut().clear())
        }

        fn signal(&self, priority: DispatcherPriority) {
            self.shared.signal(priority);
        }

        fn signal_handle(&self) -> Arc<dyn IPlatformThreadingSignal> {
            self.shared.clone()
        }

        fn current_thread_is_loop_thread(&self) -> bool {
            true
        }

        fn signaled(&self) -> &DispatcherImplEvent<Option<DispatcherPriority>> {
            &self.signaled
        }
    }

    let t = DispatcherTests::new();
    let legacy = Rc::new(Legacy {
        shared: Arc::new(Shared::default()),
        signaled: DispatcherImplEvent::new(),
        timers: Rc::new(RefCell::new(Vec::new())),
    });
    Dispatcher::initialize_ui_thread_dispatcher_legacy(legacy.clone());

    let ran = Arc::new(AtomicUsize::new(0));
    let r = ran.clone();
    t.ui_thread.post(
        move || {
            r.fetch_add(1, Ordering::SeqCst);
        },
        DispatcherPriority::NORMAL,
    );
    let r = ran.clone();
    let dispatcher = t.ui_thread.clone();
    thread::spawn(move || {
        dispatcher.post(
            move || {
                r.fetch_add(1, Ordering::SeqCst);
            },
            DispatcherPriority::NORMAL,
        )
    })
    .join()
    .unwrap();
    // One signal covers both: the second request found one pending.
    assert_eq!(*legacy.shared.signals.lock().unwrap(), [DispatcherPriority::SEND]);
    legacy.signaled.invoke(None);
    assert_eq!(ran.load(Ordering::SeqCst), 2);

    // Timers go through start_timer and are one-shot.
    let ticks = Rc::new(Cell::new(0));
    let c = ticks.clone();
    let timer = DispatcherTimer::with_callback(Duration::from_millis(20), DispatcherPriority::NORMAL, move |timer| {
        c.set(c.get() + 1);
        timer.stop();
    });
    let tick = legacy.timers.borrow().last().map(|(_, tick)| tick.clone()).unwrap();
    thread::sleep(Duration::from_millis(25));
    tick();
    t.ui_thread.run_jobs(None);
    let tick = legacy.timers.borrow().last().map(|(_, tick)| tick.clone());
    if let Some(tick) = tick {
        thread::sleep(Duration::from_millis(25));
        tick();
        t.ui_thread.run_jobs(None);
    }
    assert_eq!(ticks.get(), 1);
    assert!(!timer.is_enabled());
}
