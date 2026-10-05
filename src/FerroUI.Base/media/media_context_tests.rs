use crate::animation::{Clock, IClock, IGlobalClock, PlayState, TimeSpan};
use crate::media::MediaContext;
use crate::reactive::{IObservable, IObserver};
use crate::threading::Dispatcher;
use crate::FerroLocator;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Scope {
    _dispatcher: crate::threading::UnitTestDispatcherScope,
    locator: Rc<dyn crate::reactive::IDisposable>,
}

impl Scope {
    fn new() -> Scope {
        Scope { _dispatcher: Dispatcher::unit_test_scope(), locator: FerroLocator::enter_scope() }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        self.locator.dispose();
    }
}

struct Recorder(RefCell<Vec<TimeSpan>>);

impl IObserver<TimeSpan> for Recorder {
    fn on_next(&self, value: TimeSpan) {
        self.0.borrow_mut().push(value);
    }
}

#[test]
fn instance_is_registered_per_locator_scope() {
    let _scope = Scope::new();
    let first = MediaContext::instance();
    assert!(Rc::ptr_eq(&first, &MediaContext::instance()));
    let inner = FerroLocator::enter_scope();
    // The inner scope resolves the context of its parent.
    assert!(Rc::ptr_eq(&first, &MediaContext::instance()));
    inner.dispose();
}

#[test]
fn begin_invoke_on_render_schedules_a_render_and_runs_callbacks_queued_by_callbacks() {
    let _scope = Scope::new();
    let context = MediaContext::instance();
    let order = Rc::new(RefCell::new(Vec::new()));
    let (o1, o2) = (order.clone(), order.clone());
    context.begin_invoke_on_render(Rc::new(move || {
        o1.borrow_mut().push(1);
        let o = o2.clone();
        MediaContext::instance().begin_invoke_on_render(Rc::new(move || o.borrow_mut().push(2)));
    }));
    assert!(context.is_render_scheduled());
    assert!(order.borrow().is_empty());

    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(*order.borrow(), [1, 2]);
    assert!(!context.is_render_scheduled());
}

#[test]
#[should_panic(expected = "Infinite layout loop detected")]
fn callbacks_that_keep_requeueing_are_detected() {
    let _scope = Scope::new();
    fn requeue() {
        MediaContext::instance().begin_invoke_on_render(Rc::new(requeue));
    }
    requeue();
    MediaContext::instance().render();
}

#[test]
fn clock_pulses_observers_and_animation_frames() {
    let _scope = Scope::new();
    let context = MediaContext::instance();
    let recorder = Rc::new(Recorder(RefCell::new(Vec::new())));
    let subscription = context.clock().subscribe(recorder.clone());
    assert!(context.is_render_scheduled());
    assert!(context.media_context_clock().has_subscriptions());
    let frames = Rc::new(Cell::new(0));
    let f = frames.clone();
    context.request_animation_frame(move |_| f.set(f.get() + 1));

    context.render();
    assert_eq!(recorder.0.borrow().len(), 1);
    assert_eq!(frames.get(), 1);

    // An observer added during the frame is pulsed with the frame's time
    // before the frame ends.
    let late = Rc::new(Recorder(RefCell::new(Vec::new())));
    let late_for_callback = late.clone();
    context.begin_invoke_on_render(Rc::new(move || {
        let _ = MediaContext::instance().clock().subscribe(late_for_callback.clone());
    }));
    context.render();
    assert_eq!(recorder.0.borrow().len(), 2);
    assert_eq!(late.0.borrow().len(), 1);
    assert_eq!(late.0.borrow()[0], recorder.0.borrow()[1]);
    // An animation frame runs once.
    assert_eq!(frames.get(), 1);

    subscription.dispose();
    context.render();
    assert_eq!(recorder.0.borrow().len(), 2);
}

#[test]
fn clock_is_a_running_global_clock() {
    let _scope = Scope::new();
    let context = MediaContext::instance();
    let clock: Rc<dyn IGlobalClock> = context.clock();
    assert_eq!(PlayState::Run, clock.play_state());
    assert!(!context.media_context_clock().has_subscriptions());
}

#[test]
#[should_panic(expected = "The play state of the global clock cannot be changed.")]
fn setting_the_play_state_of_the_clock_panics() {
    let _scope = Scope::new();
    MediaContext::instance().clock().set_play_state(PlayState::Pause);
}

#[test]
fn registered_as_the_global_clock_the_context_pulses_animation_clocks() {
    let _scope = Scope::new();
    let context = MediaContext::instance();
    // What the application does when it registers its services.
    FerroLocator::current_mutable().bind::<dyn IGlobalClock>().to_constant(context.clock());
    assert!(!context.is_render_scheduled());

    let global = Clock::global_clock();
    let direct = Rc::new(Recorder(RefCell::new(Vec::new())));
    let subscription = global.subscribe(direct.clone());
    // Subscribing to the global clock schedules a render pass.
    assert!(context.is_render_scheduled());

    // A clock running on the global clock starts at zero on its first tick.
    let child = Clock::new();
    let ticks = Rc::new(Recorder(RefCell::new(Vec::new())));
    let child_subscription = child.subscribe(ticks.clone());

    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(direct.0.borrow().len(), 1);
    assert_eq!(*ticks.0.borrow(), [TimeSpan::ZERO]);

    context.render();
    assert_eq!(direct.0.borrow().len(), 2);
    assert_eq!(ticks.0.borrow().len(), 2);
    assert!(direct.0.borrow()[1] >= direct.0.borrow()[0]);
    assert_eq!(ticks.0.borrow()[1], direct.0.borrow()[1] - direct.0.borrow()[0]);

    child_subscription.dispose();
    subscription.dispose();
    child.set_play_state(PlayState::Stop);
    context.render();
    assert_eq!(direct.0.borrow().len(), 2);
    // The stopped clock unsubscribed from the global clock on that tick.
    assert!(!context.media_context_clock().has_subscriptions());
}
