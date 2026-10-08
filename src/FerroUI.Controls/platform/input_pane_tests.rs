//! The reference tests mock the top-level implementation with a mocking
//! library; here the mock window implementation of the test doubles is set up
//! with the input pane as an optional feature.

use crate::platform::{IInputPane, InputPaneState, InputPaneStateEventArgs, ITopLevelImpl};
use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControlImpl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::testing::{MockImplKind, MockWindowImpl, TestServices, UnitTestApplication};
use crate::{
    Border, ContentControl, ContentControlImpl, Control, ControlImpl, InputPaneAwareBehavior,
    InputPaneAwareDecorator, TopLevel, TopLevelImpl,
};
use ferroui_base::animation::easings::IEasing;
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::data::{BindingMode, IndexerBinding};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{ILayoutManager, LayoutableImpl};
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, StyledElementImpl, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The global clock of a test: pulses every subscriber.
struct MockGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl MockGlobalClock {
    fn new() -> Rc<Self> {
        Rc::new(Self { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) })
    }

    fn pulse(&self, time: TimeSpan) {
        self.subject.on_next(time);
    }
}

impl IObservable<TimeSpan> for MockGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for MockGlobalClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for MockGlobalClock {}

struct TestInputPane {
    open_rect: Rect,
    state: Cell<InputPaneState>,
    occluded_rect: Cell<Rect>,
    state_changed: Rc<HandlerList<dyn Fn(&InputPaneStateEventArgs)>>,
}

impl TestInputPane {
    fn new(open_rect: Rect) -> Rc<Self> {
        Rc::new(Self {
            open_rect,
            state: Cell::new(InputPaneState::Closed),
            occluded_rect: Cell::new(Rect::default()),
            state_changed: Rc::new(HandlerList::new()),
        })
    }

    fn open(&self) {
        self.open_animated(Duration::ZERO, None);
    }

    /// Opens the pane with an animation (used by the additional tests).
    fn open_animated(&self, animation_duration: Duration, easing: Option<Rc<dyn IEasing>>) {
        let old_rect = self.occluded_rect.get();
        self.occluded_rect.set(self.open_rect);

        self.state.set(InputPaneState::Open);

        let e = InputPaneStateEventArgs::with_animation(
            self.state.get(),
            Some(old_rect),
            self.occluded_rect.get(),
            animation_duration,
            easing,
        );
        for (_, handler) in self.state_changed.snapshot().iter() {
            handler(&e);
        }
    }
}

impl IInputPane for TestInputPane {
    fn state(&self) -> InputPaneState {
        self.state.get()
    }

    fn occluded_rect(&self) -> Rect {
        self.occluded_rect.get()
    }

    fn state_changed(&self, handler: Rc<dyn Fn(&InputPaneStateEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.state_changed.add(handler);
        let handlers = self.state_changed.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }
}

#[repr(C)]
struct TestTopLevel {
    base: TopLevel,
}

ferro_class!(TestTopLevel: TopLevel);
ferro_impl_classes!(
    TestTopLevel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl
);

impl TestTopLevel {
    fn new(platform_impl: &Rc<MockWindowImpl>) -> Ref<Self> {
        let platform_impl: Rc<dyn ITopLevelImpl> = platform_impl.clone();
        instantiate(Self { base: TopLevel::construct(platform_impl) })
    }
}

fn create_mock_top_level_impl(input_pane: &Rc<TestInputPane>) -> Rc<MockWindowImpl> {
    let top_level = MockWindowImpl::bare(MockImplKind::TopLevel);
    let input_pane: Rc<dyn IInputPane> = input_pane.clone();
    top_level.setup_feature::<dyn IInputPane>(input_pane);
    top_level
}

fn create_top_level_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TestTopLevel>(|x, scope| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let property = ContentControl::content_property().as_property();
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &IndexerBinding::new(x.clone().upcast(), property, BindingMode::OneWay),
        );
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

struct Target {
    clock: Rc<MockGlobalClock>,
    input_pane: Rc<TestInputPane>,
    border: Ref<Border>,
    top_level: Ref<TestTopLevel>,
}

fn create_target(behavior: InputPaneAwareBehavior, clock: &Rc<MockGlobalClock>) -> Target {
    let input_pane = TestInputPane::new(Rect::new(0.0, 200.0, 200.0, 200.0));

    let border = Border::new();

    let pane_view = InputPaneAwareDecorator::new();
    pane_view.set_child(border.clone());
    pane_view.set_height(500.0);
    pane_view.set_width(200.0);
    pane_view.set_behavior(behavior);

    let platform_impl = create_mock_top_level_impl(&input_pane);
    let top_level = TestTopLevel::new(&platform_impl);
    top_level.set_template(Some(create_top_level_template()));
    top_level.set_content(Some(Control::boxed(pane_view)));

    Target { clock: clock.clone(), input_pane, border, top_level }
}

fn start(clock: &Rc<MockGlobalClock>) -> crate::testing::UnitTestApplicationScope {
    let global_clock: Rc<dyn IGlobalClock> = clock.clone();
    UnitTestApplication::start(TestServices::real_focus().with_global_clock(global_clock))
}

#[test]
fn input_pane_aware_view_resizes_content_when_behavior_resize() {
    let clock = MockGlobalClock::new();
    let _app = start(&clock);
    let target = create_target(InputPaneAwareBehavior::Resize, &clock);

    target.top_level.layout_manager().execute_initial_layout_pass();

    assert_eq!(500.0, target.border.bounds().height);

    target.input_pane.open();

    target.clock.pulse(TimeSpan::from_seconds(5.0));

    target.top_level.layout_manager().execute_layout_pass();

    assert_eq!(300.0, target.border.bounds().height);
}

#[test]
fn input_pane_aware_view_resizes_content_when_behavior_pan() {
    let clock = MockGlobalClock::new();
    let _app = start(&clock);
    let target = create_target(InputPaneAwareBehavior::Pan, &clock);

    target.top_level.layout_manager().execute_initial_layout_pass();

    assert_eq!(0.0, target.border.bounds().top());

    target.input_pane.open();

    target.clock.pulse(TimeSpan::from_seconds(5.0));

    target.top_level.layout_manager().execute_layout_pass();

    assert_eq!(-200.0, target.border.bounds().top());
}

// ---------------------------------------------------------------------------
// Additional tests of this port (not present in the reference test file):
// the easing of the state change is used only if it is an easing class.
// ---------------------------------------------------------------------------

/// An easing that is at its end from the start.
struct StepEasing {
    is_easing_class: bool,
}

impl IEasing for StepEasing {
    fn to_shared(&self) -> std::sync::Arc<ferroui_base::animation::easings::SharedEasing> {
        std::sync::Arc::new(StepEasing { is_easing_class: self.is_easing_class })
    }

    fn ease(&self, _progress: f64) -> f64 {
        1.0
    }

    fn derives_from_easing(&self) -> bool {
        self.is_easing_class
    }
}

/// The height of the content halfway through a ten seconds animation of the
/// input pane with the given easing.
fn content_height_halfway(easing: Option<Rc<dyn IEasing>>) -> f64 {
    let clock = MockGlobalClock::new();
    let _app = start(&clock);
    let target = create_target(InputPaneAwareBehavior::Resize, &clock);

    target.top_level.layout_manager().execute_initial_layout_pass();
    assert_eq!(500.0, target.border.bounds().height);

    target.input_pane.open_animated(Duration::from_secs(10), easing);

    target.clock.pulse(TimeSpan::from_seconds(0.0));
    target.clock.pulse(TimeSpan::from_seconds(5.0));

    target.top_level.layout_manager().execute_layout_pass();

    target.border.bounds().height
}

#[test]
fn input_pane_state_change_without_easing_is_animated_linearly() {
    assert_eq!(400.0, content_height_halfway(None));
}

#[test]
fn input_pane_state_change_uses_the_easing_when_it_is_an_easing_class() {
    assert_eq!(300.0, content_height_halfway(Some(Rc::new(StepEasing { is_easing_class: true }))));
}

#[test]
fn input_pane_state_change_falls_back_to_linear_easing_when_the_easing_is_not_an_easing_class() {
    assert_eq!(400.0, content_height_halfway(Some(Rc::new(StepEasing { is_easing_class: false }))));
}

// ---------------------------------------------------------------------------
// Additional test of this port: the view observes the input pane only while
// it is attached to the visual tree.
// ---------------------------------------------------------------------------

#[test]
fn input_pane_state_changes_are_observed_only_while_the_view_is_attached() {
    let clock = MockGlobalClock::new();
    let _app = start(&clock);
    let target = create_target(InputPaneAwareBehavior::Resize, &clock);
    let handler_count = || target.input_pane.state_changed.snapshot().iter().count();

    target.top_level.layout_manager().execute_initial_layout_pass();
    let pane_view = target
        .border
        .get_visual_parent()
        .and_then(|parent| parent.cast::<InputPaneAwareDecorator>())
        .expect("the border is the child of the view");
    assert_eq!(1, handler_count());

    // Detached: the handler is removed and the padding is reset.
    target.top_level.set_content(None);
    assert_eq!(0, handler_count());

    target.input_pane.open();
    assert!(pane_view.transitions().is_none());
    assert_eq!(ferroui_base::Thickness::default(), pane_view.padding());

    // Attached again: observed once.
    target.top_level.set_content(Some(Control::boxed(pane_view.clone())));
    target.top_level.layout_manager().execute_layout_pass();
    assert_eq!(1, handler_count());

    target.clock.pulse(TimeSpan::from_seconds(5.0));
    target.top_level.layout_manager().execute_layout_pass();
    assert_eq!(300.0, target.border.bounds().height);
}
