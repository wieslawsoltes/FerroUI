//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/FromEventTest.cs` of Rx.NET.
//! Adapted: the events of the fixtures (`FromEvent`, `MyEventSource` of `FromEventPatternTest.cs`)
//! are FerroUI events (`ferroui_system::events::Event`), whose `add` returns the token that
//! `remove` takes; `FromEvent<TEventArgs>(add, remove)` is `Observable::from_event`,
//! `FromEvent(Action<Action>, Action<Action>)` is `Observable::from_event_action`, the conversion
//! overloads are `Observable::from_event_conversion`, and the `_on` forms take the scheduler. The
//! overloads with a delegate type and no conversion (`FromEvent<Action<int>, int>(add, remove)`),
//! which upstream makes the delegate of through reflection, are the closure forms of the port. The
//! sender of an event pattern is a `Sender`, the identity of the object that raises the event.
//! `FromEvent_SynchronizationContext` runs on a thread of its own with the context set, as
//! upstream's `RunWithContext` does; the subscription and the disposal, which upstream runs on new
//! threads that have no context, run on that thread with the context cleared, since the sequence
//! belongs to the thread that made it. The waits for the handler to be added and removed are checks
//! of flags, since the context `MyEventSyncCtx` runs the work it is posted at once.

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction, Scheduler};
use ferroui_reactive::{IDisposable, IObservable, Observable, ObservableExt};
use ferroui_system::events::Event;
use ferroui_system::threading::{current_synchronization_context, set_synchronization_context, SynchronizationContext};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// The identity of the object that raises an event (the sender of upstream, an `object` compared
/// by reference).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sender(usize);

impl Sender {
    fn of<T>(object: &T) -> Self {
        Self(object as *const T as usize)
    }
}

/// The handler of an event of a sender and arguments, as a handler of a FerroUI event of both.
fn pattern_handler<S: Clone + 'static, A: Clone + 'static>(handler: Rc<dyn Fn(S, A)>) -> Rc<dyn Fn(&(S, A))> {
    Rc::new(move |(sender, event_args): &(S, A)| handler(sender.clone(), event_args.clone()))
}

/// `FromEvent`: the events `A` (no arguments), `B` (an `int`) and `C` (a `MyAction`).
#[derive(Default)]
struct FromEvent {
    a: Event<()>,
    b: Event<i32>,
    c: Event<i32>,
}

impl FromEvent {
    fn on_a(&self) {
        self.a.raise(&());
    }

    fn on_b(&self, x: i32) {
        self.b.raise(&x);
    }

    fn on_c(&self, x: i32) {
        self.c.raise(&x);
    }
}

/// `MyEventArgs`.
#[derive(Clone, Debug, PartialEq)]
struct MyEventArgs {
    value: i32,
}

/// `MyEventSource`: the event `Bar` of a sender and `MyEventArgs`.
#[derive(Default)]
struct MyEventSource {
    bar: Event<(Sender, MyEventArgs)>,
}

impl MyEventSource {
    fn on_bar(&self, value: i32) {
        self.bar.raise(&(Sender::of(self), MyEventArgs { value }));
    }
}

/// `MyEventScheduler`: a scheduler that counts the work it is given and runs it at once.
struct MyEventScheduler {
    this: Weak<MyEventScheduler>,
    schedule: Box<dyn Fn()>,
}

impl MyEventScheduler {
    fn new(schedule: impl Fn() + 'static) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), schedule: Box::new(schedule) })
    }
}

impl IScheduler for MyEventScheduler {
    fn now(&self) -> SystemTime {
        Scheduler::now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        (self.schedule)();
        let scheduler: Rc<dyn IScheduler> = self.this.upgrade().expect("the scheduler is alive");
        action(&scheduler)
    }

    fn schedule_after(&self, _due_time: Duration, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        panic!("The method or operation is not implemented.")
    }

    fn schedule_at(&self, _due_time: SystemTime, _action: ScheduledAction) -> Rc<dyn IDisposable> {
        panic!("The method or operation is not implemented.")
    }
}

#[test]
fn from_event_action() {
    let fe = Rc::new(FromEvent::default());

    let (f1, f2) = (fe.clone(), fe.clone());
    let xs = Observable::from_event_action(move |h: Rc<dyn Fn()>| f1.a.add(Rc::new(move |_: &()| h())), move |token| {
        f2.a.remove(token);
    });

    fe.on_a();

    let n = Rc::new(Cell::new(0));
    let n1 = n.clone();
    let d = xs.subscribe_next(move |_| n1.set(n1.get() + 1));

    fe.on_a();
    fe.on_a();

    d.dispose();

    fe.on_a();

    assert_eq!(2, n.get());
}

/// The event `B` of `fe` as a sequence of `from_event`.
fn event_b(fe: &Rc<FromEvent>) -> Rc<dyn IObservable<i32>> {
    let (f1, f2) = (fe.clone(), fe.clone());
    Observable::from_event(move |h: Rc<dyn Fn(i32)>| f1.b.add(Rc::new(move |x: &i32| h(*x))), move |token| {
        f2.b.remove(token);
    })
}

#[test]
fn from_event_action_of_int() {
    let fe = Rc::new(FromEvent::default());

    let xs = event_b(&fe);

    fe.on_b(1);

    let n = Rc::new(Cell::new(0));
    let n1 = n.clone();
    let d = xs.subscribe_next(move |x| n1.set(n1.get() + x));

    fe.on_b(2);
    fe.on_b(3);

    d.dispose();

    fe.on_b(4);

    assert_eq!(2 + 3, n.get());
}

#[test]
fn from_event_action_of_int_specified_explicitly() {
    let fe = Rc::new(FromEvent::default());

    // `FromEvent<Action<int>, int>(h => fe.B += h, h => fe.B -= h)`: the handler of the port is
    // the closure itself.
    let xs = event_b(&fe);

    fe.on_b(1);

    let n = Rc::new(Cell::new(0));
    let n1 = n.clone();
    let d = xs.subscribe_next(move |x| n1.set(n1.get() + x));

    fe.on_b(2);
    fe.on_b(3);

    d.dispose();

    fe.on_b(4);

    assert_eq!(2 + 3, n.get());
}

#[test]
fn from_event_action_of_int_specified_explicitly_trivial_conversion() {
    let fe = Rc::new(FromEvent::default());

    let (f1, f2) = (fe.clone(), fe.clone());
    let xs = Observable::from_event_conversion(
        |h: Rc<dyn Fn(i32)>| h,
        move |h: Rc<dyn Fn(i32)>| f1.b.add(Rc::new(move |x: &i32| h(*x))),
        move |token| {
            f2.b.remove(token);
        },
    );

    fe.on_b(1);

    let n = Rc::new(Cell::new(0));
    let n1 = n.clone();
    let d = xs.subscribe_next(move |x| n1.set(n1.get() + x));

    fe.on_b(2);
    fe.on_b(3);

    d.dispose();

    fe.on_b(4);

    assert_eq!(2 + 3, n.get());
}

#[test]
fn from_event_my_action() {
    let fe = Rc::new(FromEvent::default());

    // `MyAction` is the handler type of the event `C`, `Fn(&i32)`.
    let (f1, f2) = (fe.clone(), fe.clone());
    let xs = Observable::from_event_conversion(
        |h: Rc<dyn Fn(i32)>| -> Rc<dyn Fn(&i32)> { Rc::new(move |x: &i32| h(*x)) },
        move |h: Rc<dyn Fn(&i32)>| f1.c.add(h),
        move |token| {
            f2.c.remove(token);
        },
    );

    fe.on_c(1);

    let n = Rc::new(Cell::new(0));
    let n1 = n.clone();
    let d = xs.subscribe_next(move |x| n1.set(n1.get() + x));

    fe.on_c(2);
    fe.on_c(3);

    d.dispose();

    fe.on_c(4);

    assert_eq!(2 + 3, n.get());
}

#[test]
fn from_event_implicit_publish() {
    let src = Rc::new(MyEventSource::default());

    let add_count = Rc::new(Cell::new(0));
    let rem_count = Rc::new(Cell::new(0));

    let (s1, s2, a, r) = (src.clone(), src.clone(), add_count.clone(), rem_count.clone());
    let xs = Observable::from_event_pattern_on(
        move |h: Rc<dyn Fn(Sender, MyEventArgs)>| {
            a.set(a.get() + 1);
            s1.bar.add(pattern_handler(h))
        },
        move |token| {
            s2.bar.remove(token);
            r.set(r.get() + 1);
        },
        Scheduler::immediate(),
    );

    assert_eq!(0, add_count.get());
    assert_eq!(0, rem_count.get());

    src.on_bar(41);

    let fst = Rc::new(RefCell::new(Vec::new()));
    let f = fst.clone();
    let d1 = xs.subscribe_next(move |e| f.borrow_mut().push(e.event_args.value));

    assert_eq!(1, add_count.get());
    assert_eq!(0, rem_count.get());

    src.on_bar(42);

    assert_eq!(vec![42], *fst.borrow());

    d1.dispose();

    assert_eq!(1, add_count.get());
    assert_eq!(1, rem_count.get());

    let snd = Rc::new(RefCell::new(Vec::new()));
    let s = snd.clone();
    let d2 = xs.subscribe_next(move |e| s.borrow_mut().push(e.event_args.value));

    assert_eq!(2, add_count.get());
    assert_eq!(1, rem_count.get());

    src.on_bar(43);

    assert_eq!(vec![42], *fst.borrow());
    assert_eq!(vec![43], *snd.borrow());

    let thd = Rc::new(RefCell::new(Vec::new()));
    let t = thd.clone();
    let d3 = xs.subscribe_next(move |e| t.borrow_mut().push(e.event_args.value));

    assert_eq!(2, add_count.get());
    assert_eq!(1, rem_count.get());

    src.on_bar(44);

    assert_eq!(vec![42], *fst.borrow());
    assert_eq!(vec![43, 44], *snd.borrow());
    assert_eq!(vec![44], *thd.borrow());

    d2.dispose();

    assert_eq!(2, add_count.get());
    assert_eq!(1, rem_count.get());

    src.on_bar(45);

    assert_eq!(vec![42], *fst.borrow());
    assert_eq!(vec![43, 44], *snd.borrow());
    assert_eq!(vec![44, 45], *thd.borrow());

    d3.dispose();

    assert_eq!(2, add_count.get());
    assert_eq!(2, rem_count.get());

    src.on_bar(46);

    assert_eq!(vec![42], *fst.borrow());
    assert_eq!(vec![43, 44], *snd.borrow());
    assert_eq!(vec![44, 45], *thd.borrow());
}

#[test]
fn from_event_synchronization_context() {
    let flags = run_with_context(MyEventSyncCtx::new(), |ctx| {
        let src = Rc::new(MyEventSource::default());

        let add_ctx: Rc<RefCell<Option<Arc<dyn SynchronizationContext>>>> = Rc::new(RefCell::new(None));
        let rem_ctx: Rc<RefCell<Option<Arc<dyn SynchronizationContext>>>> = Rc::new(RefCell::new(None));

        let add_evt = Rc::new(Cell::new(false));
        let rem_evt = Rc::new(Cell::new(false));

        let (s1, s2, a, r, ae, re) = (src.clone(), src.clone(), add_ctx.clone(), rem_ctx.clone(), add_evt.clone(), rem_evt.clone());
        let xs = Observable::from_event_pattern(
            move |h: Rc<dyn Fn(Sender, MyEventArgs)>| {
                *a.borrow_mut() = current_synchronization_context();
                let token = s1.bar.add(pattern_handler(h));
                ae.set(true);
                token
            },
            move |token| {
                *r.borrow_mut() = current_synchronization_context();
                s2.bar.remove(token);
                re.set(true);
            },
        );

        assert!(add_ctx.borrow().is_none());
        assert!(rem_ctx.borrow().is_none());

        let res = Rc::new(RefCell::new(Vec::new()));

        // `new Thread(...)`: no context is current where the sequence is subscribed to.
        set_synchronization_context(None);
        let before_subscribe_null = current_synchronization_context().is_none();
        let rs = res.clone();
        let d = xs.subscribe_next(move |e| rs.borrow_mut().push(e.event_args.value));
        let after_subscribe_null = current_synchronization_context().is_none();
        set_synchronization_context(Some(ctx.clone()));

        assert!(add_evt.get());

        let subscribe_on_ctx = is_context(&add_ctx.borrow(), &ctx);

        src.on_bar(42);
        let fst_next = *res.borrow() == vec![42];

        src.on_bar(43);
        let snd_next = *res.borrow() == vec![42, 43];

        // `new Thread(...)`: no context is current where the subscription is disposed.
        set_synchronization_context(None);
        let before_dispose_null = current_synchronization_context().is_none();
        d.dispose();
        let after_dispose_null = current_synchronization_context().is_none();
        set_synchronization_context(Some(ctx.clone()));

        assert!(rem_evt.get());

        let dispose_on_ctx = is_context(&rem_ctx.borrow(), &ctx);

        src.on_bar(44);
        let thd_next = *res.borrow() == vec![42, 43];

        [
            before_subscribe_null,
            subscribe_on_ctx,
            after_subscribe_null,
            fst_next,
            snd_next,
            thd_next,
            before_dispose_null,
            dispose_on_ctx,
            after_dispose_null,
        ]
    });

    let [before_subscribe_null, subscribe_on_ctx, after_subscribe_null, fst_next, snd_next, thd_next, before_dispose_null, dispose_on_ctx, after_dispose_null] =
        flags;

    assert!(before_subscribe_null);
    assert!(subscribe_on_ctx);
    assert!(after_subscribe_null);

    assert!(fst_next);
    assert!(snd_next);
    assert!(thd_next);

    assert!(before_dispose_null);
    assert!(dispose_on_ctx);
    assert!(after_dispose_null);
}

/// `RunWithContext`: runs `run` on a new thread whose current context is `ctx`, and waits for it.
fn run_with_context<R: Send + 'static>(ctx: Arc<MyEventSyncCtx>, run: impl FnOnce(Arc<MyEventSyncCtx>) -> R + Send + 'static) -> R {
    std::thread::spawn(move || {
        set_synchronization_context(Some(ctx.clone()));
        let result = run(ctx);
        set_synchronization_context(None);
        result
    })
    .join()
    .unwrap()
}

/// Whether `context` is `ctx` (`ReferenceEquals`).
fn is_context(context: &Option<Arc<dyn SynchronizationContext>>, ctx: &Arc<MyEventSyncCtx>) -> bool {
    context.as_ref().is_some_and(|context| std::ptr::addr_eq(Arc::as_ptr(context), Arc::as_ptr(ctx)))
}

/// `MyEventSyncCtx` (of `FromEventPatternTest.cs`): runs the work it is posted at once, as the
/// current context of the thread, counting the posts.
struct MyEventSyncCtx {
    this: std::sync::Weak<MyEventSyncCtx>,
    post_count: AtomicUsize,
}

impl MyEventSyncCtx {
    fn new() -> Arc<Self> {
        Arc::new_cyclic(|this| Self { this: this.clone(), post_count: AtomicUsize::new(0) })
    }

    /// Runs `work` with this context as the current one, then restores the previous one, also
    /// when the work panics (the `try`/`finally` of `Post`).
    fn run_as_current(&self, work: impl FnOnce()) {
        struct Restore(Option<Arc<dyn SynchronizationContext>>);
        impl Drop for Restore {
            fn drop(&mut self) {
                set_synchronization_context(self.0.take());
            }
        }

        let old = current_synchronization_context();
        let this: Arc<dyn SynchronizationContext> = self.this.upgrade().expect("the context is alive while it is posted to");
        set_synchronization_context(Some(this));
        let _restore = Restore(old);
        self.post_count.fetch_add(1, Ordering::SeqCst);
        work();
    }
}

impl SynchronizationContext for MyEventSyncCtx {
    fn post(&self, work: Box<dyn FnOnce() + Send>) {
        self.run_as_current(work);
    }

    /// The `Send` of the base class: the work runs at once.
    fn send(&self, work: Box<dyn FnOnce() + Send>) {
        work();
    }

    fn post_local(&self, work: Box<dyn FnOnce()>) {
        self.run_as_current(work);
    }

    fn send_local(&self, work: Box<dyn FnOnce()>) {
        work();
    }
}

#[test]
fn from_event_scheduler1() {
    run_with_scheduler(|s, add, remove| Observable::from_event_on(move |_h: Rc<dyn Fn(MyEventArgs)>| add(), move |_: ()| remove(), s));
}

#[test]
fn from_event_scheduler2() {
    run_with_scheduler(|s, add, remove| Observable::from_event_action_on(move |_h: Rc<dyn Fn()>| add(), move |_: ()| remove(), s));
}

#[test]
fn from_event_scheduler3() {
    // `FromEvent<Action<MyEventArgs>, MyEventArgs>(add, remove, s)`: the handler of the port is the
    // closure itself.
    run_with_scheduler(|s, add, remove| Observable::from_event_on(move |_h: Rc<dyn Fn(MyEventArgs)>| add(), move |_: ()| remove(), s));
}

#[test]
fn from_event_scheduler4() {
    run_with_scheduler(|s, add, remove| {
        Observable::from_event_conversion_on(
            |_h: Rc<dyn Fn(MyEventArgs)>| -> Rc<dyn Fn()> { Rc::new(|| {}) },
            move |_h: Rc<dyn Fn()>| add(),
            move |_: ()| remove(),
            s,
        )
    });
}

/// `RunWithScheduler`: the handler is added on the scheduler at the first subscription only, and
/// removed on it at the disposal of the last subscription.
fn run_with_scheduler<T: 'static>(run: impl FnOnce(Rc<dyn IScheduler>, Rc<dyn Fn()>, Rc<dyn Fn()>) -> Rc<dyn IObservable<T>>) {
    let n = Rc::new(Cell::new(0));
    let a = Rc::new(Cell::new(0));
    let r = Rc::new(Cell::new(0));

    let counter = n.clone();
    let s: Rc<dyn IScheduler> = MyEventScheduler::new(move || counter.set(counter.get() + 1));

    let (added, removed) = (a.clone(), r.clone());
    let add: Rc<dyn Fn()> = Rc::new(move || added.set(added.get() + 1));
    let rem: Rc<dyn Fn()> = Rc::new(move || removed.set(removed.get() + 1));

    let xs = run(s, add, rem);

    assert_eq!(0, n.get());
    assert_eq!(0, a.get());
    assert_eq!(0, r.get());

    let d1 = xs.subscribe_ignore();
    assert_eq!(1, n.get());
    assert_eq!(1, a.get());
    assert_eq!(0, r.get());

    let d2 = xs.subscribe_ignore();
    assert_eq!(1, n.get());
    assert_eq!(1, a.get());
    assert_eq!(0, r.get());

    d1.dispose();
    assert_eq!(1, n.get());
    assert_eq!(1, a.get());
    assert_eq!(0, r.get());

    d2.dispose();
    assert_eq!(2, n.get());
    assert_eq!(1, a.get());
    assert_eq!(1, r.get());
}

// Not ported: FromEvent_ArgumentChecking - argument validation
