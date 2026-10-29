//! Lifecycle tests for the portable configuration entrance and its weak pen owner.
use super::*;
use crate::test_support::{test_scope, TestRoot};
use ferroui_controls::Flyout;
use crate::InkToolbarBallpointPenButton;
use ferroui_base::animation::PlayState;
use ferroui_base::reactive::{IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;

struct ManualClock {
    ticks: LightweightSubject<TimeSpan>,
    state: Cell<PlayState>,
}

impl ManualClock {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            ticks: LightweightSubject::new(),
            state: Cell::new(PlayState::Run),
        })
    }
    fn pulse(&self, milliseconds: f64) {
        self.ticks
            .on_next(TimeSpan::from_milliseconds(milliseconds));
    }
}

impl IObservable<TimeSpan> for ManualClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.ticks.subscribe(observer)
    }
}

impl IClock for ManualClock {
    fn play_state(&self) -> PlayState {
        self.state.get()
    }
    fn set_play_state(&self, state: PlayState) {
        self.state.set(state);
    }
}

#[test]
fn configuration_entrance_staggers_children_and_restores_values_on_detach_and_completion() {
    let _scope = test_scope();
    let clock = ManualClock::new();
    let configuration = InkToolbarPenConfigurationControl::new();
    let first = configuration.children().get(0);
    let last = configuration
        .children()
        .get(configuration.children().count() - 1);
    first.set_opacity(0.6);
    let original: Rc<dyn ITransform> = TranslateTransform::with_offset(5.0, 7.0).into();
    first.set_render_transform(Some(original));
    configuration.set_clock(Some(clock.clone()));
    let root = TestRoot::with_child(&configuration);
    assert!(clock.ticks.has_observers());
    clock.pulse(0.0);
    clock.pulse(0.0);
    assert_eq!(first.opacity(), 0.0);
    assert_eq!(last.opacity(), 0.0);
    assert_eq!(first.render_transform().unwrap().value().m31, 45.0);
    clock.pulse(50.0);
    assert!(first.opacity() > 0.0 && first.opacity() < 0.6);
    assert_eq!(
        last.opacity(),
        0.0,
        "Later sections wait for their stagger delay"
    );
    let x = first.render_transform().unwrap().value().m31;
    assert!(x > 5.0 && x < 45.0);

    // New application values written during the animation remain its base values.
    first.set_opacity(0.8);
    let replacement: Rc<dyn ITransform> = TranslateTransform::with_offset(8.0, 9.0).into();
    first.set_render_transform(Some(replacement.clone()));
    root.set_child(None);
    assert_eq!(first.opacity(), 0.8);
    assert!(Rc::ptr_eq(&first.render_transform().unwrap(), &replacement));
    assert_eq!(last.opacity(), 1.0);
    assert!(configuration.entrance.borrow().is_none());
    // The framework's stopped child clocks release their parent on its next pulse.
    clock.pulse(1000.0);
    assert!(!clock.ticks.has_observers());

    root.set_child(&configuration);
    clock.pulse(1000.0);
    clock.pulse(1000.0);
    assert_eq!(first.opacity(), 0.0, "Reattachment starts a fresh entrance");
    clock.pulse(1400.0);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(configuration.entrance.borrow().is_none());
    assert_eq!(first.opacity(), 0.8);
    assert!(Rc::ptr_eq(&first.render_transform().unwrap(), &replacement));
    assert_eq!(last.opacity(), 1.0);
    clock.pulse(1401.0);
    assert!(!clock.ticks.has_observers());
    root.set_child(None);
}

#[test]
fn dropping_configuration_cancels_entrance_without_retaining_children_or_overrides() {
    let _scope = test_scope();
    let clock = ManualClock::new();
    let configuration = InkToolbarPenConfigurationControl::new();
    configuration.set_clock(Some(clock.clone()));
    let first = configuration.children().get(0);
    first.set_opacity(0.75);
    let weak = configuration.downgrade();
    // Exercise the same running handles independently of a root's detach notification.
    configuration.start_entrance();
    clock.pulse(0.0);
    clock.pulse(30.0);
    assert!(first.opacity() < 0.75);
    drop(configuration);
    assert!(weak.upgrade().is_none());
    assert_eq!(first.opacity(), 0.75);
    assert!(first.render_transform().is_none());
    clock.pulse(1000.0);
    assert!(!clock.ticks.has_observers());
}

#[test]
fn pen_owned_flyout_configuration_has_no_reverse_ownership_cycle() {
    let _scope = test_scope();
    let pen = InkToolbarBallpointPenButton::new();
    let configuration = InkToolbarPenConfigurationControl::for_pen(&pen);
    assert_eq!(configuration.pen_button(), Some(pen.clone().upcast()));
    let flyout = Flyout::new();
    flyout.set_content(Some(Control::boxed(&configuration)));
    pen.set_flyout(&flyout);
    let weak_pen = pen.downgrade();
    let weak_configuration = configuration.downgrade();
    drop(configuration);
    drop(flyout);
    drop(pen);
    assert!(weak_pen.upgrade().is_none());
    assert!(weak_configuration.upgrade().is_none());
}
#[test]
fn checking_a_radio_item_unchecks_the_others_of_its_group() {
    let _scope = test_scope();
    let panel = ferroui_controls::StackPanel::new();
    let items: Vec<_> = (0..3)
        .map(|_| {
            let item = InkToolbarFlyoutItem::new();
            item.set_kind(InkToolbarFlyoutItemKind::Radio);
            panel.children().add(&item);
            item
        })
        .collect();
    let check = InkToolbarFlyoutItem::new();
    check.set_kind(InkToolbarFlyoutItemKind::Check);
    check.set_is_checked(Some(true));
    panel.children().add(&check);
    items[0].set_is_checked(Some(true));
    items[2].set_is_checked(Some(true));
    assert_eq!(items[0].is_checked(), Some(false));
    assert_eq!(items[1].is_checked(), Some(false));
    assert_eq!(items[2].is_checked(), Some(true));
    // Another kind is another group.
    assert_eq!(check.is_checked(), Some(true));
}
