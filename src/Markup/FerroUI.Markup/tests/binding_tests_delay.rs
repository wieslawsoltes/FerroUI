//! Ported from the upstream `BindingTests_Delay`.

use super::binding_tests::Source;
use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::{BindingExpressionBase, BindingMode, UpdateSourceTrigger};
use ferroui_base::threading::{Dispatcher, DispatcherImplEvent, IDispatcherImpl, IDispatcherSignal};
use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
use ferroui_base::{FerroLocator, Ref};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const DELAY_MILLISECONDS: i32 = 10;
const INITIAL_FOO_VALUE: &str = "foo";

struct ManualSignal {
    signaled: AtomicBool,
}

impl IDispatcherSignal for ManualSignal {
    fn signal(&self) {
        self.signaled.store(true, Ordering::SeqCst);
    }
}

/// A dispatcher implementation whose clock and timer are driven by the test.
struct ManualTimerDispatcher {
    now: Cell<i64>,
    signal: Arc<ManualSignal>,
    signaled: DispatcherImplEvent,
    timer: DispatcherImplEvent,
}

impl ManualTimerDispatcher {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            now: Cell::new(0),
            signal: Arc::new(ManualSignal { signaled: AtomicBool::new(false) }),
            signaled: DispatcherImplEvent::new(),
            timer: DispatcherImplEvent::new(),
        })
    }

    /// Runs the jobs the dispatcher asked to be signaled for. (The dispatcher
    /// must not be re-entered from `signal`, so the signal is delivered
    /// here.)
    fn run_signaled(&self) {
        while self.signal.signaled.swap(false, Ordering::SeqCst) {
            self.signaled.raise();
        }
    }

    fn raise_timer_event(&self) {
        self.timer.raise();
        self.run_signaled();
    }
}

impl IDispatcherImpl for ManualTimerDispatcher {
    fn current_thread_is_loop_thread(&self) -> bool {
        true
    }

    fn signal(&self) {
        self.signal.signal();
    }

    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        self.signal.clone()
    }

    fn signaled(&self) -> &DispatcherImplEvent {
        &self.signaled
    }

    fn timer(&self) -> &DispatcherImplEvent {
        &self.timer
    }

    fn now(&self) -> i64 {
        self.now.get()
    }

    fn update_timer(&self, _due_time_in_ms: Option<i64>) {}
}

/// The state the upstream test class sets up in its constructor.
struct Fixture {
    dispatcher: Rc<ManualTimerDispatcher>,
    source: Rc<Source>,
    target: Ref<TextBox>,
    binding_expr: Rc<dyn BindingExpressionBase>,
    // Dropped last: resets the dispatcher of the thread.
    _scope: ferroui_base::threading::UnitTestDispatcherScope,
}

impl Fixture {
    fn new() -> Self {
        let scope = Dispatcher::unit_test_scope();
        let keyboard: Rc<dyn IKeyboardDevice> = KeyboardDevice::new();
        FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(keyboard);
        let dispatcher = ManualTimerDispatcher::new();
        Dispatcher::initialize_ui_thread_dispatcher(dispatcher.clone());

        let source = Source::with_foo(Some(INITIAL_FOO_VALUE));
        let target = TextBox::new_focusable();
        target.set_data_context(Some(source.clone()));

        let binding = Binding::with_path_and_mode("Foo", BindingMode::TwoWay);
        binding.set_delay(DELAY_MILLISECONDS);

        let binding_expr = target.bind_binding(TextBox::text_property(), &binding);

        assert_eq!(source.foo(), target.text());

        Self { dispatcher, source, target, binding_expr, _scope: scope }
    }

    fn set_time_and_execute_timers(&self, time: i64) {
        self.dispatcher.now.set(time);
        self.dispatcher.raise_timer_event();
    }
}

#[test]
fn delayed_binding_should_set_value_only_after_delay_elapsed() {
    let f = Fixture::new();
    f.target.set_text(Some("bar"));

    assert_eq!(f.source.foo().as_deref(), Some(INITIAL_FOO_VALUE));

    f.set_time_and_execute_timers(DELAY_MILLISECONDS as i64 / 2);

    assert_eq!(f.source.foo().as_deref(), Some(INITIAL_FOO_VALUE));

    f.set_time_and_execute_timers(DELAY_MILLISECONDS as i64 + 1);

    assert_eq!(f.source.foo().as_deref(), Some("bar"));
}

#[test]
fn delayed_binding_should_not_set_value_after_being_disposed() {
    let f = Fixture::new();
    f.target.set_text(Some("bar"));

    assert_eq!(f.source.foo().as_deref(), Some(INITIAL_FOO_VALUE));

    f.binding_expr.dispose();

    f.set_time_and_execute_timers(DELAY_MILLISECONDS as i64 + 1);

    assert_eq!(f.source.foo().as_deref(), Some(INITIAL_FOO_VALUE));
}

#[test]
fn delayed_binding_should_restart_if_value_changes_during_delay() {
    let f = Fixture::new();
    f.target.set_text(Some("bar"));

    assert_eq!(f.source.foo().as_deref(), Some(INITIAL_FOO_VALUE));

    f.set_time_and_execute_timers(DELAY_MILLISECONDS as i64 / 2);

    f.target.set_text(Some("baz"));

    // A new value was set half-way through the delay, so the delay is still
    // in effect at this timestamp.
    f.set_time_and_execute_timers(DELAY_MILLISECONDS as i64 + 1);

    assert_eq!(f.source.foo().as_deref(), Some(INITIAL_FOO_VALUE));

    f.set_time_and_execute_timers(DELAY_MILLISECONDS as i64 * 2);

    assert_eq!(f.source.foo().as_deref(), Some("baz"));
}

#[test]
fn delayed_binding_should_not_execute_if_value_returns_to_original() {
    let f = Fixture::new();
    f.target.set_text(Some("bar"));

    assert_eq!(f.source.foo().as_deref(), Some(INITIAL_FOO_VALUE));

    f.set_time_and_execute_timers(DELAY_MILLISECONDS as i64 / 2);

    f.target.set_text(Some(INITIAL_FOO_VALUE));

    f.set_time_and_execute_timers(DELAY_MILLISECONDS as i64 * 2);

    assert_eq!(f.source.foo().as_deref(), Some(INITIAL_FOO_VALUE));
    assert_eq!(f.source.foo_set_count(), 1);
}

#[test]
fn delayed_binding_update_source_call_should_update_source_immediately() {
    let f = Fixture::new();
    f.target.set_text(Some("bar"));
    f.binding_expr.update_source();
    assert_eq!(f.source.foo().as_deref(), Some("bar"));
}

#[test]
fn delayed_binding_update_trigger_lost_focus_should_update_source_immediately() {
    let f = Fixture::new();
    let second_box = TextBox::new_focusable();
    let panel = Panel::new();
    panel.add(&f.target);
    panel.add(&second_box);
    let _root = TestRoot::with_child(&panel);

    let binding = Binding::with_path_and_mode("Foo", BindingMode::TwoWay);
    binding.set_delay(DELAY_MILLISECONDS);
    binding.set_update_source_trigger(UpdateSourceTrigger::LostFocus);
    f.target.bind_binding(TextBox::text_property(), &binding);

    assert!(f.target.focus());

    f.target.set_text(Some("bar"));
    assert_eq!(f.source.foo().as_deref(), Some(INITIAL_FOO_VALUE));

    assert!(second_box.focus());

    assert_eq!(f.source.foo().as_deref(), Some("bar"));
}

#[test]
fn delayed_binding_one_way_to_source_data_context_change_should_update_source_immediately() {
    let f = Fixture::new();
    let binding = Binding::with_path_and_mode("Foo", BindingMode::OneWayToSource);
    binding.set_delay(DELAY_MILLISECONDS);
    f.target.bind_binding(TextBlock::text_property(), &binding);

    f.target.set_text(Some("bar"));

    let new_source = Source::new();
    f.target.set_data_context(Some(new_source.clone()));

    assert_eq!(new_source.foo().as_deref(), Some("bar"));
}

#[test]
fn delayed_binding_should_update_target_immediately() {
    let f = Fixture::new();
    f.source.set_foo(Some(s("bar")));
    assert_eq!(f.target.text().as_deref(), Some("bar"));
}
