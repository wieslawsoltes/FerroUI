//! Tests of the animation system, ported from the upstream animation tests
//! (one file per upstream test class), plus tests specific to this port.
//!
//! The control library is a separate crate, so the tests use minimal classes
//! in place of the controls of the tests these were ported from.

#![allow(dead_code)]

use crate::animation::{IClock, IGlobalClock, ITransition, PlayState, TimeSpan};
use crate::layout::{ILayoutManager, ILayoutRoot, LayoutManager, Layoutable, LayoutableImpl};
use crate::media::IBrush;
use crate::reactive::{Disposable, IDisposable, IObservable, IObserver};
use crate::input::{FocusManager, IInputRoot, InputElement, InputElementImpl};
use crate::interactivity::InteractiveImpl;
use crate::rendering::{IHitTester, IPresentationSource, IRenderer, ManagedHitTester};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

mod animatable_tests;
mod animation_iteration_tests;
mod brush_transition_tests;
mod effect_tests;
mod page_transition_tests;
mod key_spline_tests;
mod spring_tests;
mod style_animation_tests;
mod transitions_tests;

pub(crate) fn seconds(value: f64) -> TimeSpan {
    TimeSpan::from_seconds(value)
}

/// A clock that is stepped by the test. It has a single observer: the last
/// subscriber.
pub(crate) struct TestClock {
    cur_time: Cell<TimeSpan>,
    observer: RefCell<Option<Rc<dyn IObserver<TimeSpan>>>>,
    play_state: Cell<PlayState>,
}

impl TestClock {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self { cur_time: Cell::new(TimeSpan::ZERO), observer: RefCell::new(None), play_state: Cell::new(PlayState::Run) })
    }

    pub(crate) fn as_clock(self: &Rc<Self>) -> Rc<dyn IClock> {
        self.clone()
    }

    /// Sets the absolute time.
    pub(crate) fn step(&self, time: TimeSpan) {
        let observer = self.observer.borrow().clone();
        if let Some(observer) = observer {
            observer.on_next(time);
        }
    }

    /// Advances the time.
    pub(crate) fn pulse(&self, time: TimeSpan) {
        self.cur_time.set(self.cur_time.get() + time);
        self.step(self.cur_time.get());
    }

    pub(crate) fn has_observer(&self) -> bool {
        self.observer.borrow().is_some()
    }
}

impl IObservable<TimeSpan> for TestClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        *self.observer.borrow_mut() = Some(observer);
        Disposable::empty()
    }
}

impl IClock for TestClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

/// The global clock of a test: pulses every subscriber.
pub(crate) struct MockGlobalClock {
    subject: crate::reactive::LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl MockGlobalClock {
    pub(crate) fn pulse(&self, time: TimeSpan) {
        self.subject.on_next(time);
    }

    pub(crate) fn has_observers(&self) -> bool {
        self.subject.has_observers()
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

/// Registers a global clock for the test (the test harness runs every test
/// on its own thread, with its own service locator).
pub(crate) fn start() -> Rc<MockGlobalClock> {
    let clock =
        Rc::new(MockGlobalClock { subject: crate::reactive::LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
    let service: Rc<dyn IGlobalClock> = clock.clone();
    FerroLocator::current_mutable().bind::<dyn IGlobalClock>().to_constant(service);
    clock
}

/// A control: a layoutable with a background brush.
#[repr(C)]
pub struct Border {
    base: Layoutable,
}

ferro_class!(Border: Layoutable);
ferro_impl_classes!(Border: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl);

impl Border {
    ferro_property!(pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<Border, _>("Background", None)
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Layoutable::construct() })
    }

    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }
}

/// The root of a visual tree that is attached to a presentation source.
#[repr(C)]
pub struct TestRoot {
    base: InputElement,
    source: RefCell<Option<Rc<TestSource>>>,
}

ferro_class!(TestRoot: InputElement);
ferro_impl_classes!(TestRoot: FerroObjectImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl);

impl StyledElementImpl for TestRoot {
    fn is_logical_root(_this: &Self) -> bool {
        true
    }
}

impl TestRoot {
    pub fn new() -> Ref<Self> {
        let root = instantiate(Self { base: InputElement::construct(), source: RefCell::new(None) });
        let source = TestSource::new(root.clone());
        *root.source.borrow_mut() = Some(source.clone());
        root.set_presentation_source_for_root_visual(Some(source));
        root
    }

    pub fn with_child<T: ObjectType + Upcast<StyledElement> + Upcast<Visual>>(child: &Ref<T>) -> Ref<Self> {
        let root = Self::new();
        root.set_child(child);
        root
    }

    pub fn set_child<T: ObjectType + Upcast<StyledElement> + Upcast<Visual>>(&self, child: &Ref<T>) {
        self.logical_children().add(child.clone().upcast());
        self.visual_children().add(child.clone().upcast());
    }

    pub fn remove_child<T: ObjectType + Upcast<StyledElement> + Upcast<Visual>>(&self, child: &Ref<T>) {
        self.visual_children().remove(&child.clone().upcast());
        self.logical_children().remove(&child.clone().upcast());
    }
}

/// Makes `child` a logical and visual child of `parent`.
pub(crate) fn add_child<P, T>(parent: &Ref<P>, child: &Ref<T>)
where
    P: ObjectType + Upcast<Visual>,
    T: ObjectType + Upcast<StyledElement> + Upcast<Visual>,
{
    let parent: Ref<Visual> = parent.clone().upcast();
    parent.logical_children().add(child.clone().upcast());
    parent.visual_children().add(child.clone().upcast());
}

struct TestRenderer;

impl IRenderer for TestRenderer {
    fn diagnostics(&self) -> Rc<crate::rendering::RendererDiagnostics> {
        crate::rendering::RendererDiagnostics::new()
    }
    fn scene_invalidated(
        &self,
        _handler: Rc<dyn Fn(&crate::rendering::SceneInvalidatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn try_get_render_interface_feature(&self, _feature_type: std::any::TypeId) -> Option<crate::rendering::composition::RenderInterfaceFeature> {
        None
    }
    fn add_dirty(&self, _visual: &Visual) {}
    fn recalculate_children(&self, _visual: &Visual) {}
    fn resized(&self, _size: Size) {}
    fn paint(&self, _rect: Rect) {}
    fn start(&self) {}
    fn stop(&self) {}
    fn dispose(&self) {}
}

struct TestSource {
    this: Weak<TestSource>,
    root: WeakRef<TestRoot>,
    renderer: Rc<TestRenderer>,
    layout_manager: RefCell<Option<Rc<LayoutManager>>>,
}

impl TestSource {
    fn new(root: Ref<TestRoot>) -> Rc<Self> {
        let source = Rc::new_cyclic(|this: &Weak<TestSource>| TestSource {
            this: this.clone(),
            root: root.downgrade(),
            renderer: Rc::new(TestRenderer),
            layout_manager: RefCell::new(None),
        });
        let as_layout_root: Rc<dyn ILayoutRoot> = source.clone();
        *source.layout_manager.borrow_mut() = Some(LayoutManager::new(Rc::downgrade(&as_layout_root)));
        source
    }
}

impl IPresentationSource for TestSource {
    fn root_visual(&self) -> Option<Ref<Visual>> {
        self.root.upgrade().map(Ref::upcast)
    }
    fn render_scaling(&self) -> f64 {
        1.0
    }
    fn renderer(&self) -> Rc<dyn IRenderer> {
        self.renderer.clone()
    }
    fn layout_root(&self) -> Rc<dyn ILayoutRoot> {
        self.this.upgrade().unwrap()
    }
    fn hit_tester(&self) -> Rc<dyn IHitTester> {
        Rc::new(ManagedHitTester::new())
    }
    fn input_root(&self) -> Rc<dyn IInputRoot> {
        self.this.upgrade().unwrap()
    }
    fn client_size(&self) -> Size {
        Size::new(800.0, 600.0)
    }
}

impl IInputRoot for TestSource {
    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        None
    }
    fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_pointer_over_element(&self, _value: Option<Ref<InputElement>>) {}
    fn cursor_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_cursor_element(&self, _value: Option<Ref<InputElement>>) {}
    fn root_element(&self) -> Ref<InputElement> {
        self.root.upgrade().unwrap().upcast()
    }
    fn focus_root(&self) -> Ref<InputElement> {
        self.root.upgrade().unwrap().upcast()
    }
    fn pointer_over_invalidated(&self) {}
}

impl ILayoutRoot for TestSource {
    fn layout_scaling(&self) -> f64 {
        1.0
    }
    fn layout_manager(&self) -> Rc<dyn ILayoutManager> {
        self.layout_manager.borrow().clone().unwrap()
    }
    fn root_visual(&self) -> Ref<Layoutable> {
        self.root.upgrade().unwrap().upcast()
    }
}

/// A call to [`ITransition::apply`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ApplyCall {
    pub old_value: f64,
    pub new_value: f64,
}

/// A hand-written mock of a transition of an `f64` property: records the
/// calls to `apply` and returns a subscription that counts its disposals.
pub(crate) struct MockTransition {
    property: Cell<&'static FerroProperty>,
    pub calls: RefCell<Vec<ApplyCall>>,
    pub disposed: Rc<Cell<u32>>,
    pub on_apply: RefCell<Option<Rc<dyn Fn(&Animatable)>>>,
}

use crate::animation::Animatable;

impl MockTransition {
    pub(crate) fn new(property: &'static FerroProperty) -> Rc<Self> {
        Rc::new(Self {
            property: Cell::new(property),
            calls: RefCell::new(Vec::new()),
            disposed: Rc::new(Cell::new(0)),
            on_apply: RefCell::new(None),
        })
    }

    pub(crate) fn handle(self: &Rc<Self>) -> Rc<dyn ITransition> {
        self.clone()
    }

    pub(crate) fn clear(&self) {
        self.calls.borrow_mut().clear();
    }

    pub(crate) fn call_count(&self) -> usize {
        self.calls.borrow().len()
    }

    pub(crate) fn was_applied(&self, old_value: f64, new_value: f64) -> bool {
        self.calls.borrow().contains(&ApplyCall { old_value, new_value })
    }
}

impl ITransition for MockTransition {
    fn apply(
        &self,
        control: &Animatable,
        _clock: Rc<dyn IClock>,
        old_value: &BoxedValue,
        new_value: &BoxedValue,
    ) -> Rc<dyn IDisposable> {
        let old_value: &dyn AnyValue = &**old_value;
        let new_value: &dyn AnyValue = &**new_value;
        self.calls.borrow_mut().push(ApplyCall {
            old_value: *old_value.downcast_ref::<f64>().unwrap(),
            new_value: *new_value.downcast_ref::<f64>().unwrap(),
        });
        let on_apply = self.on_apply.borrow().clone();
        if let Some(on_apply) = on_apply {
            on_apply(control);
        }
        let disposed = self.disposed.clone();
        // Every disposal is counted, like the calls on a mock.
        struct Sub(Rc<Cell<u32>>);
        impl IDisposable for Sub {
            fn dispose(&self) {
                self.0.set(self.0.get() + 1)
            }
        }
        Rc::new(Sub(disposed))
    }

    fn property(&self) -> &'static FerroProperty {
        self.property.get()
    }

    fn set_property(&self, value: &'static FerroProperty) {
        self.property.set(value)
    }
}

#[track_caller]
pub(crate) fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!((actual - expected).abs() <= tolerance, "expected {expected} ± {tolerance}, got {actual}");
}

#[track_caller]
pub(crate) fn assert_panics(f: impl FnOnce()) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    assert!(result.is_err(), "expected a panic");
}
