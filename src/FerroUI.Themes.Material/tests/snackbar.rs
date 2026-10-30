//! Request-lifetime, real-template, automation and controlled-frame regressions.
use crate::{
    register_types, MaterialMotionRole, MaterialMotionScheme, MaterialSnackbarDuration,
    MaterialSnackbarHost, MaterialSnackbarMessage, MaterialSnackbarResult,
    MaterialSnackbarTimeoutPolicy, MaterialTheme,
};
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::{Dispatcher, DispatcherTimer};
use ferroui_base::{ObjectType, Ref, Visual};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::{AutomationLiveSetting, AutomationProperties};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Button, Control, Grid, TextBlock, Window};
use std::cell::Cell;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

struct TestClock {
    subject: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}
impl IObservable<TimeSpan> for TestClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}
impl IClock for TestClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, value: PlayState) {
        self.state.set(value);
    }
}
impl IGlobalClock for TestClock {}

fn application() -> (UnitTestApplicationScope, Rc<TestClock>) {
    register_types();
    let clock = Rc::new(TestClock {
        subject: LightweightSubject::new(),
        state: Cell::new(PlayState::Run),
    });
    let app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_global_clock(clock.clone())
            .with_platform_settings(crate::motion::MotionTestSettings::new())
            .with_theme(|| MaterialTheme::new().as_style()),
    );
    (app, clock)
}

fn host(host: &MaterialSnackbarHost) -> Ref<Window> {
    let window = Window::new();
    window.set_width(480.0);
    window.set_height(320.0);
    window.set_content(Some(Control::boxed(host.to_ref())));
    window.show();
    window.update_layout();
    window
}

fn named<T: ObjectType>(visual: &Visual, name: &str) -> Ref<T> {
    visual
        .get_visual_descendants()
        .filter(|node| node.name().as_deref() == Some(name))
        .find_map(|node| node.cast::<T>())
        .unwrap_or_else(|| panic!("missing snackbar template part {name}"))
}

fn timer(interval: Duration) -> Rc<DispatcherTimer> {
    Dispatcher::timers_for_unit_tests()
        .into_iter()
        .find(|timer| timer.interval() == interval)
        .expect("snackbar timeout timer")
}

#[test]
fn duration_defaults_are_native_and_explicit_duration_wins() {
    assert_eq!(
        MaterialSnackbarDuration::Short,
        MaterialSnackbarMessage::new("Saved").effective_duration()
    );
    assert_eq!(
        MaterialSnackbarDuration::Indefinite,
        MaterialSnackbarMessage::new("Deleted")
            .with_action("Undo")
            .effective_duration()
    );
    assert_eq!(
        MaterialSnackbarDuration::Long,
        MaterialSnackbarMessage::new("Deleted")
            .with_action("Undo")
            .with_duration(MaterialSnackbarDuration::Long)
            .effective_duration()
    );
    assert_eq!(
        Some(Duration::from_secs(4)),
        MaterialSnackbarDuration::Short.timeout()
    );
    assert_eq!(
        Some(Duration::from_secs(10)),
        MaterialSnackbarDuration::Long.timeout()
    );
    assert_eq!(None, MaterialSnackbarDuration::Indefinite.timeout());
}

#[test]
fn identical_messages_have_distinct_fifo_identity_and_one_terminal_result() {
    let (_app, _) = application();
    let target = MaterialSnackbarHost::new();
    target.set_reduced_motion(true);
    let first = target.show(MaterialSnackbarMessage::new("Same").with_action("Undo"));
    let second = target.show(MaterialSnackbarMessage::new("Same").with_action("Undo"));
    assert_ne!(first.id(), second.id());
    assert_eq!(Some(first.id()), target.current_id());
    assert_eq!(1, target.queued_count());
    assert!(!second.perform_action(), "queued request is not actionable");
    assert!(first.perform_action());
    assert_eq!(
        Some(MaterialSnackbarResult::ActionPerformed),
        first.result()
    );
    assert_eq!(Some(second.id()), target.current_id());
    assert!(!first.dismiss());
    assert!(!first.cancel());
    assert!(second.dismiss());
    assert_eq!(Some(MaterialSnackbarResult::Dismissed), second.result());
    assert!(!target.is_open());
}

#[test]
fn queued_cancellation_preserves_current_and_skips_cancelled_item() {
    let (_app, _) = application();
    let target = MaterialSnackbarHost::new();
    let first = target.show("First");
    let cancelled = target.show("Cancelled");
    let third = target.show("Third");
    assert!(cancelled.cancel());
    assert_eq!(Some(MaterialSnackbarResult::Cancelled), cancelled.result());
    assert_eq!(Some(first.id()), target.current_id());
    assert_eq!(1, target.queued_count());
    assert!(first.dismiss());
    assert_eq!(Some(third.id()), target.current_id());
}

#[test]
fn discarded_handle_keeps_message_but_cancelled_future_removes_request() {
    let (_app, _) = application();
    let target = MaterialSnackbarHost::new();
    drop(target.show("Fire and forget"));
    assert!(target.is_open());
    let queued = target.show("Awaited");
    let mut future = queued.clone().into_future();
    assert_eq!(
        Poll::Pending,
        Pin::new(&mut future).poll(&mut Context::from_waker(Waker::noop()))
    );
    drop(future);
    assert_eq!(Some(MaterialSnackbarResult::Cancelled), queued.result());
    assert_eq!(0, target.queued_count());
    target.dismiss_current();
    assert!(!target.is_open());
}

#[test]
fn awaiting_finished_request_returns_result_without_cancelling_successor() {
    let (_app, _) = application();
    let target = MaterialSnackbarHost::new();
    let first = target.show("First");
    let second = target.show("Second");
    first.dismiss();
    let mut future = first.into_future();
    assert_eq!(
        Poll::Ready(MaterialSnackbarResult::Dismissed),
        Pin::new(&mut future).poll(&mut Context::from_waker(Waker::noop()))
    );
    drop(future);
    assert_eq!(Some(second.id()), target.current_id());
}

#[test]
fn clear_cancels_all_requests_without_showing_an_intermediate_item() {
    let (_app, _) = application();
    let target = MaterialSnackbarHost::new();
    target.set_reduced_motion(true);
    let first = target.show("First");
    let second = target.show("Second");
    let third = target.show("Third");
    target.clear();
    assert!(!target.is_open());
    assert_eq!(0, target.queued_count());
    for request in [first, second, third] {
        assert_eq!(Some(MaterialSnackbarResult::Cancelled), request.result());
    }
}

#[test]
fn host_and_visual_weak_ownership_releases_pending_requests() {
    let (_app, _) = application();
    let target = MaterialSnackbarHost::new();
    let weak = target.downgrade();
    let first = target.show("First");
    let visual = target.current_snackbar().unwrap();
    let second = target.show("Second");
    drop(target);
    assert!(
        weak.upgrade().is_none(),
        "retaining a request or its visual must not retain the host"
    );
    assert_eq!(Some(MaterialSnackbarResult::Cancelled), first.result());
    assert_eq!(Some(MaterialSnackbarResult::Cancelled), second.result());
    assert_eq!("First", visual.message());
}

#[test]
fn lifetime_starts_on_display_and_each_queued_item_gets_its_complete_timeout() {
    let (_app, _) = application();
    let target = MaterialSnackbarHost::new();
    target.set_reduced_motion(true);
    let first = target.show("First");
    let second = target
        .show(MaterialSnackbarMessage::new("Second").with_duration(MaterialSnackbarDuration::Long));
    assert!(
        Dispatcher::timers_for_unit_tests().is_empty(),
        "unattached messages are not readable yet"
    );
    let _window = host(&target);
    let first_timer = timer(Duration::from_secs(4));
    assert!(!Dispatcher::timers_for_unit_tests()
        .iter()
        .any(|timer| timer.interval() == Duration::from_secs(10)));
    Dispatcher::force_fire_timer_for_unit_tests(&first_timer);
    assert_eq!(Some(MaterialSnackbarResult::Dismissed), first.result());
    assert_eq!(Some(second.id()), target.current_id());
    let second_timer = timer(Duration::from_secs(10));
    assert!(!Rc::ptr_eq(&first_timer, &second_timer));
    assert!(!first.dismiss());
    assert_eq!(None, second.result());
    Dispatcher::force_fire_timer_for_unit_tests(&second_timer);
    assert_eq!(Some(MaterialSnackbarResult::Dismissed), second.result());
    assert!(!target.is_open());
}

#[test]
fn action_defaults_to_indefinite_and_template_buttons_complete_the_correct_request() {
    let (_app, _) = application();
    let target = MaterialSnackbarHost::new();
    target.set_reduced_motion(true);
    let request = target.show(
        MaterialSnackbarMessage::new("Removed")
            .with_action("Undo")
            .with_dismiss_action(true),
    );
    let window = host(&target);
    assert!(Dispatcher::timers_for_unit_tests().is_empty());
    let visual = target.current_snackbar().unwrap();
    let action: Ref<Button> = named(&visual, "PART_ActionButton");
    let dismiss: Ref<Button> = named(&visual, "PART_DismissButton");
    assert!(action.is_visible() && dismiss.is_visible());
    assert!(action.bounds().height >= 48.0);
    assert!(dismiss.bounds().width >= 48.0 && dismiss.bounds().height >= 48.0);
    action.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        Some(MaterialSnackbarResult::ActionPerformed),
        request.result()
    );
    let next = target.show(MaterialSnackbarMessage::new("Notice").with_dismiss_action(true));
    window.update_layout();
    dismiss.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        None,
        next.result(),
        "old button must not dismiss a successor"
    );
    let dismiss: Ref<Button> = named(&target.current_snackbar().unwrap(), "PART_DismissButton");
    dismiss.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(Some(MaterialSnackbarResult::Dismissed), next.result());
}

#[test]
fn snackbar_is_a_polite_named_status_and_action_on_new_line_moves_real_controls() {
    let (_app, _) = application();
    let target = MaterialSnackbarHost::new();
    target.set_reduced_motion(true);
    let _request = target.show(
        MaterialSnackbarMessage::new("A message")
            .with_action("A longer action")
            .with_dismiss_action(true)
            .with_action_on_new_line(true),
    );
    let _window = host(&target);
    let visual = target.current_snackbar().unwrap();
    let peer = ControlAutomationPeer::create_peer_for_element(&visual);
    assert_eq!(
        AutomationControlType::StatusBar,
        peer.get_automation_control_type()
    );
    assert_eq!(AutomationLiveSetting::Polite, peer.get_live_setting());
    assert_eq!("A message", peer.get_name());
    let message: Ref<TextBlock> = named(&visual, "PART_Message");
    let action: Ref<Button> = named(&visual, "PART_ActionButton");
    assert_eq!(3, Grid::get_column_span(&message));
    assert_eq!(1, Grid::get_row(&action));
    assert!(action.bounds().y >= message.bounds().bottom());
}

struct ExtendedTimeout;
impl MaterialSnackbarTimeoutPolicy for ExtendedTimeout {
    fn recommended_timeout(&self, original: Duration, contains_controls: bool) -> Option<Duration> {
        assert_eq!(Duration::from_secs(4), original);
        assert!(contains_controls);
        Some(Duration::from_secs(30))
    }
}

#[test]
fn accessibility_recommendation_controls_readable_lifetime_independently_of_motion() {
    let (_app, clock) = application();
    let target = MaterialSnackbarHost::new();
    target.set_timeout_policy(Some(Rc::new(ExtendedTimeout)));
    let _window = host(&target);
    let request = target.show(
        MaterialSnackbarMessage::new("Notice")
            .with_action("Review")
            .with_duration(MaterialSnackbarDuration::Short),
    );
    let lifetime = timer(Duration::from_secs(30));
    target.set_reduced_motion(true);
    assert_eq!(None, request.result());
    assert!(lifetime.is_enabled());
    assert!(!clock.subject.has_observers());
    assert_eq!(1.0, target.current_snackbar().unwrap().opacity());
    Dispatcher::force_fire_timer_for_unit_tests(&lifetime);
    assert_eq!(Some(MaterialSnackbarResult::Dismissed), request.result());
}

#[test]
fn entry_motion_samples_fast_effects_opacity_and_fast_spatial_scale() {
    let (_app, clock) = application();
    let target = MaterialSnackbarHost::new();
    let _window = host(&target);
    target.show(
        MaterialSnackbarMessage::new("Notice").with_duration(MaterialSnackbarDuration::Indefinite),
    );
    let visual = target.current_snackbar().unwrap();
    assert_eq!(0.0, visual.opacity());
    assert!(clock.subject.has_observers());
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(0.05));
    let scheme = MaterialMotionScheme::Expressive;
    let opacity = scheme
        .spring(MaterialMotionRole::FastEffects)
        .sample(0.05, 0.0, 1.0, 0.0)
        .value;
    let scale = scheme
        .spring(MaterialMotionRole::FastSpatial)
        .sample(0.05, 0.8, 1.0, 0.0)
        .value;
    assert!((visual.opacity() - opacity).abs() < 1e-10);
    assert!((visual.render_transform().unwrap().value().m11 - scale).abs() < 1e-10);
    clock.subject.on_next(TimeSpan::from_seconds(2.0));
    assert_eq!(1.0, visual.opacity());
    assert!(
        !clock.subject.has_observers(),
        "settled snackbar must not request frames"
    );
}

#[test]
fn outgoing_visual_survives_exit_fade_without_announcing_or_accepting_actions() {
    let (_app, clock) = application();
    let target = MaterialSnackbarHost::new();
    let _window = host(&target);
    let first = target.show(MaterialSnackbarMessage::new("First").with_action("Undo"));
    let outgoing = target.current_snackbar().unwrap();
    clock.subject.on_next(TimeSpan::from_seconds(0.0));
    clock.subject.on_next(TimeSpan::from_seconds(1.0));
    let second = target.show("Second");
    first.dismiss();
    assert_eq!(Some(second.id()), target.current_id());
    assert!(outgoing.is_attached_to_visual_tree());
    assert_eq!(
        AutomationLiveSetting::Off,
        AutomationProperties::get_live_setting(&outgoing)
    );
    assert!(!outgoing.is_hit_test_visible());
    assert!(!first.perform_action());
    clock.subject.on_next(TimeSpan::from_seconds(1.0));
    clock.subject.on_next(TimeSpan::from_seconds(2.0));
    assert!(!outgoing.is_attached_to_visual_tree());
    assert!(!clock.subject.has_observers());
}

#[test]
fn hidden_and_detached_hosts_release_both_clocks_without_losing_the_queue() {
    let (_app, clock) = application();
    let target = MaterialSnackbarHost::new();
    let window = host(&target);
    let first = target.show("First");
    let second = target.show("Second");
    assert!(clock.subject.has_observers());
    assert!(!Dispatcher::timers_for_unit_tests().is_empty());
    target.set_is_visible(false);
    assert!(!clock.subject.has_observers());
    assert!(Dispatcher::timers_for_unit_tests().is_empty());
    assert_eq!(Some(first.id()), target.current_id());
    target.set_is_visible(true);
    assert!(clock.subject.has_observers());
    assert!(!Dispatcher::timers_for_unit_tests().is_empty());
    window.set_content(None);
    window.update_layout();
    assert!(!clock.subject.has_observers());
    assert!(Dispatcher::timers_for_unit_tests().is_empty());
    assert_eq!(None, first.result());
    assert_eq!(None, second.result());
    assert_eq!(1, target.queued_count());
}
