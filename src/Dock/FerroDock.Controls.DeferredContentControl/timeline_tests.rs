use crate::presentation::IDeferredContentPresentationTarget;
use crate::{
    DeferredContentPresentationBudgetMode as Mode, DeferredContentPresentationSettings as Settings,
    DeferredContentPresentationTimeline as Timeline,
};
use ferroui_base::animation::TimeSpan;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .use_headless(FerroHeadlessPlatformOptions::default())
        .after_setup(|_| {
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_fluent::FluentTheme::new().as_style())
        })
}
pub(crate) const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-deferred-content",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
pub(crate) fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct Target {
    timeline: RefCell<Weak<Timeline>>,
    ready: Cell<bool>,
    retain: Cell<bool>,
    attempts: Cell<usize>,
    successes: Cell<usize>,
    delay: Cell<Duration>,
    max_attempts: Cell<usize>,
    action: RefCell<Option<Box<dyn Fn()>>>,
}
impl Target {
    fn new(ready: bool) -> Rc<Self> {
        Rc::new(Self {
            timeline: RefCell::new(Weak::new()),
            ready: Cell::new(ready),
            retain: Cell::new(true),
            attempts: Cell::new(0),
            successes: Cell::new(0),
            delay: Cell::new(Duration::ZERO),
            max_attempts: Cell::new(usize::MAX),
            action: RefCell::new(None),
        })
    }
    fn erased(self: &Rc<Self>) -> Rc<dyn IDeferredContentPresentationTarget> {
        self.clone()
    }
}
impl IDeferredContentPresentationTarget for Target {
    fn enqueued_timeline(&self) -> Option<Rc<Timeline>> {
        self.timeline.borrow().upgrade()
    }
    fn set_enqueued_timeline(&self, value: Option<Rc<Timeline>>) {
        *self.timeline.borrow_mut() = value.map_or_else(Weak::new, |t| Rc::downgrade(&t));
    }
    fn retain_pending_presentation_on_failure(&self) -> bool {
        self.retain.get()
    }
    fn apply_deferred_presentation(&self) -> bool {
        if !self.delay.get().is_zero() {
            std::thread::sleep(self.delay.get());
        }
        self.attempts.set(self.attempts.get() + 1);
        assert!(
            self.attempts.get() <= self.max_attempts.get(),
            "target was revisited in one pass"
        );
        if let Some(action) = self.action.borrow().as_ref() {
            action();
        }
        if self.ready.get() {
            self.successes.set(self.successes.get() + 1);
            true
        } else {
            false
        }
    }
}
fn manual() -> Rc<Timeline> {
    let t = Timeline::new();
    t.set_auto_schedule(false);
    t
}
#[test]
fn source_defaults_clamping_and_default_settings_share_one_timeline() {
    run(|| {
        let t = Timeline::new();
        assert_eq!(t.budget_mode(), Mode::ItemCount);
        assert_eq!(t.max_presentations_per_pass(), 8);
        assert_eq!(
            t.max_realization_time_per_pass(),
            TimeSpan::from_milliseconds(10.0)
        );
        assert_eq!(t.initial_delay(), TimeSpan::ZERO);
        assert_eq!(t.follow_up_delay(), TimeSpan::from_milliseconds(1.0));
        assert_eq!(t.reveal_duration(), TimeSpan::from_milliseconds(90.0));
        assert!(t.auto_schedule());
        t.set_budget_mode(Mode::RealizationTime);
        assert_eq!(t.budget_mode(), Mode::RealizationTime);
        for value in [0, -12, i32::MIN, 2] {
            t.set_max_presentations_per_pass(value);
            assert_eq!(t.max_presentations_per_pass(), value.max(1));
        }
        for value in [
            TimeSpan::from_ticks(-1),
            TimeSpan::ZERO,
            TimeSpan::from_milliseconds(3.0),
        ] {
            t.set_initial_delay(value);
            t.set_follow_up_delay(value);
            t.set_reveal_duration(value);
            t.set_max_realization_time_per_pass(value);
            assert_eq!(t.initial_delay(), value.max(TimeSpan::ZERO));
            assert_eq!(t.follow_up_delay(), value.max(TimeSpan::ZERO));
            assert_eq!(t.reveal_duration(), value.max(TimeSpan::ZERO));
            assert_eq!(t.max_realization_time_per_pass(), value.max(TimeSpan::ZERO));
        }
        let shared = Settings::default_timeline();
        assert!(Rc::ptr_eq(&shared, &Settings::default_timeline()));
        Settings::set_budget_mode(Mode::RealizationTime);
        Settings::set_max_presentations_per_pass(3);
        Settings::set_max_realization_time_per_pass(TimeSpan::ZERO);
        Settings::set_initial_delay(TimeSpan::from_ticks(-2));
        Settings::set_follow_up_delay(TimeSpan::ZERO);
        Settings::set_reveal_duration(TimeSpan::ZERO);
        assert_eq!(Settings::budget_mode(), Mode::RealizationTime);
        assert_eq!(Settings::max_presentations_per_pass(), 3);
        assert_eq!(Settings::max_realization_time_per_pass(), TimeSpan::ZERO);
        assert_eq!(Settings::initial_delay(), TimeSpan::ZERO);
        assert_eq!(Settings::follow_up_delay(), TimeSpan::ZERO);
        assert_eq!(Settings::reveal_duration(), TimeSpan::ZERO);
        Settings::set_budget_mode(Mode::ItemCount);
        Settings::set_max_presentations_per_pass(8);
        Settings::set_max_realization_time_per_pass(TimeSpan::from_milliseconds(10.0));
        Settings::set_follow_up_delay(TimeSpan::from_milliseconds(1.0));
        Settings::set_reveal_duration(TimeSpan::from_milliseconds(90.0));
    });
}
#[test]
fn source_not_ready_targets_do_not_starve_ready_targets() {
    run(|| {
        let t = manual();
        t.set_max_presentations_per_pass(2);
        let targets = [
            Target::new(false),
            Target::new(false),
            Target::new(true),
            Target::new(true),
            Target::new(true),
        ];
        for target in &targets {
            t.enqueue(target.erased(), TimeSpan::ZERO, 0);
        }
        assert_eq!(t.pending_count(), 5);
        t.flush_pending_batch_for_testing();
        assert_eq!(
            targets.iter().map(|v| v.attempts.get()).collect::<Vec<_>>(),
            vec![1, 1, 1, 1, 0]
        );
        assert_eq!(t.pending_count(), 3);
        targets[0].ready.set(true);
        targets[1].ready.set(true);
        t.flush_pending_batch_for_testing();
        assert_eq!(targets.iter().map(|v| v.successes.get()).sum::<usize>(), 4);
        assert_eq!(t.pending_count(), 1);
        t.flush_pending_batch_for_testing();
        assert_eq!(targets.iter().map(|v| v.successes.get()).sum::<usize>(), 5);
        assert_eq!(t.pending_count(), 0);
    });
}
#[test]
fn source_never_revisits_blocked_target_within_one_item_count_pass() {
    run(|| {
        let t = manual();
        t.set_max_presentations_per_pass(2);
        let targets = [Target::new(false), Target::new(false), Target::new(false)];
        for target in &targets {
            target.max_attempts.set(1);
            t.enqueue(target.erased(), TimeSpan::ZERO, 0);
        }
        t.flush_pending_batch_for_testing();
        for target in &targets {
            assert_eq!(target.attempts.get(), 1);
            t.remove(&target.erased());
        }
        assert_eq!(t.pending_count(), 0);
    });
}
#[test]
fn source_drops_not_ready_real_target_without_polling() {
    run(|| {
        let t = manual();
        let target = Target::new(false);
        target.retain.set(false);
        t.enqueue(target.erased(), TimeSpan::ZERO, 0);
        assert_eq!(t.pending_count(), 1);
        t.flush_pending_batch_for_testing();
        assert_eq!(target.attempts.get(), 1);
        assert_eq!(t.pending_count(), 0);
        assert!(target.enqueued_timeline().is_none());
        t.flush_pending_batch_for_testing();
        assert_eq!(target.attempts.get(), 1);
    });
}
#[test]
fn source_time_budget_counts_elapsed_work_instead_of_completed_item_limit() {
    run(|| {
        let t = manual();
        t.set_budget_mode(Mode::RealizationTime);
        t.set_max_presentations_per_pass(1);
        t.set_max_realization_time_per_pass(TimeSpan::from_milliseconds(10.0));
        let targets = [Target::new(true), Target::new(true), Target::new(true)];
        targets[0].delay.set(Duration::from_millis(25));
        for target in &targets {
            t.enqueue(target.erased(), TimeSpan::ZERO, 0);
        }
        t.flush_pending_batch_for_testing();
        assert_eq!(targets[0].successes.get(), 1);
        assert_eq!(targets[1].successes.get(), 0);
        assert_eq!(targets[2].successes.get(), 0);
        assert_eq!(t.pending_count(), 2);
        t.flush_pending_batch_for_testing();
        assert_eq!(targets[1].successes.get(), 1);
        assert_eq!(targets[2].successes.get(), 1);
        assert_eq!(t.pending_count(), 0);
    });
}
#[test]
fn source_low_order_first_and_zero_time_budget_still_attempts_one_target() {
    run(|| {
        let t = manual();
        t.set_budget_mode(Mode::RealizationTime);
        t.set_max_realization_time_per_pass(TimeSpan::ZERO);
        let slow = Target::new(true);
        slow.delay.set(Duration::from_millis(20));
        let fast = Target::new(true);
        t.enqueue(slow.erased(), TimeSpan::ZERO, 10);
        t.enqueue(fast.erased(), TimeSpan::ZERO, -10);
        t.flush_pending_batch_for_testing();
        assert_eq!(fast.successes.get(), 1);
        assert_eq!(slow.successes.get(), 0);
        assert_eq!(t.pending_count(), 1);
        t.flush_pending_batch_for_testing();
        assert_eq!(slow.successes.get(), 1);
        assert_eq!(t.pending_count(), 0);
    });
}
#[test]
fn reenqueue_updates_same_entry_due_order_and_sequence_without_duplicate_realization() {
    run(|| {
        let t = manual();
        t.set_max_presentations_per_pass(1);
        let a = Target::new(true);
        let b = Target::new(true);
        t.enqueue(a.erased(), TimeSpan::ZERO, 0);
        t.enqueue(b.erased(), TimeSpan::ZERO, 0);
        t.enqueue(a.erased(), TimeSpan::ZERO, 0);
        assert_eq!(t.pending_count(), 2);
        t.flush_pending_batch_for_testing();
        assert_eq!(b.successes.get(), 1);
        assert_eq!(a.successes.get(), 0);
        t.flush_pending_batch_for_testing();
        assert_eq!(a.successes.get(), 1);
    });
}
#[test]
fn callback_removal_and_replacement_skip_old_snapshot_identity() {
    run(|| {
        let t = manual();
        let a = Target::new(true);
        let b = Target::new(true);
        let weak = Rc::downgrade(&t);
        let next = b.erased();
        a.action.replace(Some(Box::new(move || {
            let t = weak.upgrade().unwrap();
            t.remove(&next);
            t.enqueue(next.clone(), TimeSpan::ZERO, 0);
        })));
        t.enqueue(a.erased(), TimeSpan::ZERO, 0);
        t.enqueue(b.erased(), TimeSpan::ZERO, 0);
        t.flush_pending_batch_for_testing();
        assert_eq!(a.successes.get(), 1);
        assert_eq!(b.attempts.get(), 0);
        assert_eq!(t.pending_count(), 1);
        t.flush_pending_batch_for_testing();
        assert_eq!(b.successes.get(), 1);
    });
}
#[test]
fn late_removal_from_old_timeline_keeps_current_timeline_reference() {
    run(|| {
        let a = manual();
        let b = manual();
        let target = Target::new(true);
        a.enqueue(target.erased(), TimeSpan::ZERO, 0);
        b.enqueue(target.erased(), TimeSpan::ZERO, 0);
        a.remove(&target.erased());
        assert!(Rc::ptr_eq(&target.enqueued_timeline().unwrap(), &b));
        b.remove(&target.erased());
        assert!(target.enqueued_timeline().is_none());
    });
}
#[test]
fn delayed_auto_schedule_reschedules_earlier_work_and_stale_posts_do_not_reapply() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch_task(
            || async {
                let t = Timeline::new();
                let late = Target::new(true);
                let early = Target::new(true);
                t.enqueue(late.erased(), TimeSpan::from_milliseconds(35.0), 0);
                t.enqueue(early.erased(), TimeSpan::ZERO, 0);
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(early.successes.get(), 1);
                assert_eq!(late.successes.get(), 0);
                let until = Instant::now() + Duration::from_secs(2);
                while late.successes.get() == 0 && Instant::now() < until {
                    dispatcher_delay(Duration::from_millis(2)).await;
                    Dispatcher::ui_thread().run_jobs(None);
                }
                assert_eq!(late.successes.get(), 1);
                assert_eq!(t.pending_count(), 0);
                t.enqueue(early.erased(), TimeSpan::ZERO, 0);
                t.remove(&early.erased());
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(early.successes.get(), 1);
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
pub(crate) async fn dispatcher_delay(duration: Duration) {
    let ready = Rc::new(Cell::new(false));
    let waker = Rc::new(RefCell::new(None::<std::task::Waker>));
    let completed = ready.clone();
    let wake = waker.clone();
    ferroui_base::threading::DispatcherTimer::run_once(
        move || {
            completed.set(true);
            if let Some(waker) = wake.borrow_mut().take() {
                waker.wake();
            }
        },
        duration,
        ferroui_base::threading::DispatcherPriority::BACKGROUND,
    );
    std::future::poll_fn(move |cx| {
        if ready.get() {
            std::task::Poll::Ready(())
        } else {
            *waker.borrow_mut() = Some(cx.waker().clone());
            std::task::Poll::Pending
        }
    })
    .await;
}
