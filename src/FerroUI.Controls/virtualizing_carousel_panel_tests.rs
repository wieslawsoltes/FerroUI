//! The reference tests mock the page transition contract with a mocking
//! library; here `MockTransition` records the calls and returns the task it
//! was set up with (a completed task by default, as the mock does).
//!
//! The reference tests run the continuations of asynchronous methods by
//! executing the callbacks posted to a test synchronization context; here
//! continuations are jobs of the dispatcher of the test, so running its jobs
//! stands for both steps.

use crate::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter};
use crate::primitives::ScrollBarVisibility;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{string_of, test_scope, TestRoot, TestScope};
use crate::{
    Button, Canvas, Carousel, Control, ItemsControl, ItemsSource, Label, Panel, ScrollViewer,
    VirtualizingCarouselPanel,
};
use ferroui_base::animation::{
    CrossFade, IClock, IGlobalClock, IPageTransition, IProgressPageTransition, PageSlide, PageTransitionItem,
    PlayState, SlideAxis, TimeSpan,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{SwipeGestureEndedEventArgs, SwipeGestureEventArgs};
use ferroui_base::layout::Layoutable;
use ferroui_base::media::{ITransform, ScaleTransform, Transform, TransformGroup, TranslateTransform};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::{CancellationToken, Dispatcher, DispatcherTask};
use ferroui_base::{FerroLocator, Ref, Size, StaticType, Vector, Visual};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

fn start() -> TestScope {
    test_scope()
}

/// The global clock of a test: pulses every subscriber.
struct MockGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl MockGlobalClock {
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

/// Starts a test whose animations run on a clock pulsed by the test.
fn start_with_clock() -> (TestScope, Rc<MockGlobalClock>) {
    let scope = test_scope();
    let clock = Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
    let service: Rc<dyn IGlobalClock> = clock.clone();
    FerroLocator::current_mutable().bind::<dyn IGlobalClock>().to_constant(service);
    (scope, clock)
}

/// Runs the continuations of the asynchronous methods and the jobs of the
/// dispatcher.
fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

struct Target {
    panel: Ref<VirtualizingCarouselPanel>,
    carousel: Ref<Carousel>,
    /// The root owns the tree.
    _root: Ref<TestRoot>,
}

struct Options {
    transition: Option<Rc<dyn IPageTransition>>,
    selected_index: Option<i32>,
    viewport_fraction: f64,
    client_size: Option<Size>,
}

impl Default for Options {
    fn default() -> Self {
        Self { transition: None, selected_index: None, viewport_fraction: 1.0, client_size: None }
    }
}

fn create_target(items: impl Into<ItemsSource>) -> Target {
    create_target_with(items, Options::default())
}

fn create_target_with_transition(items: impl Into<ItemsSource>, transition: Rc<dyn IPageTransition>) -> Target {
    create_target_with(items, Options { transition: Some(transition), ..Default::default() })
}

fn create_target_with(items: impl Into<ItemsSource>, options: Options) -> Target {
    let size = options.client_size.unwrap_or(Size::new(400.0, 300.0));
    let carousel = Carousel::new();
    carousel.set_items_source(Some(items.into()));
    carousel.set_template(Some(carousel_template()));
    carousel.set_page_transition(options.transition);
    carousel.set_viewport_fraction(options.viewport_fraction);
    carousel.set_width(size.width);
    carousel.set_height(size.height);

    if let Some(selected_index) = options.selected_index {
        carousel.set_selected_index(selected_index);
    }

    let root = TestRoot::with_child(carousel.clone());
    root.set_client_size(size);
    root.layout_manager().execute_initial_layout_pass();
    let panel = carousel.presenter().unwrap().panel().unwrap().cast::<VirtualizingCarouselPanel>().unwrap();
    Target { panel, carousel, _root: root }
}

fn carousel_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        scroll_viewer.set_template(Some(scroll_viewer_template()));
        scroll_viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroll_viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**ns))));
        scroll_viewer.register_in_name_scope(&**ns).upcast()
    })
}

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));

        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.upcast()
    })
}

fn layout(c: &Layoutable) {
    if let Some(layout_manager) = c.get_layout_manager() {
        layout_manager.execute_layout_pass();
    }
}

fn strs(values: &[&str]) -> ItemsSource {
    ItemsSource::from_strs(values.iter().copied())
}

fn string_list(values: &[&str]) -> Rc<FerroList<String>> {
    Rc::new(FerroList::from_items(values.iter().map(|x| x.to_string())))
}

fn controls_source(items: &[Ref<Control>]) -> ItemsSource {
    ItemsSource::from_items(items.iter().map(|x| Some(Control::boxed(x.clone()))))
}

/// The child as a content presenter (exactly that class).
fn content_presenter(control: &Ref<Control>) -> Ref<ContentPresenter> {
    assert!(std::ptr::eq(control.get_type(), <ContentPresenter as StaticType>::TYPE));
    control.clone().cast::<ContentPresenter>().unwrap()
}

fn content_of(presenter: &ContentPresenter) -> Option<String> {
    presenter.content().and_then(|content| string_of(&content))
}

fn content(value: &str) -> Option<String> {
    Some(value.to_string())
}

/// The children of the panel that are content presenters.
fn content_presenters(panel: &VirtualizingCarouselPanel) -> Vec<Ref<ContentPresenter>> {
    panel.children().to_vec().into_iter().filter_map(|x| x.cast::<ContentPresenter>()).collect()
}

/// The realized containers that are content presenters, with their content.
fn realized_by_content(panel: &VirtualizingCarouselPanel) -> Vec<(String, Ref<ContentPresenter>)> {
    panel
        .get_realized_containers()
        .unwrap()
        .into_iter()
        .filter_map(|x| x.cast::<ContentPresenter>())
        .map(|x| (content_of(&x).unwrap(), x))
        .collect()
}

fn realized<'a>(realized: &'a [(String, Ref<ContentPresenter>)], content: &str) -> &'a Ref<ContentPresenter> {
    &realized.iter().find(|x| x.0 == content).unwrap_or_else(|| panic!("'{content}' is not realized.")).1
}

/// Asserts equality to six decimal places.
fn assert_close(expected: f64, actual: f64) {
    assert!((expected - actual).abs() < 0.000_000_5, "Expected {expected} but got {actual}.");
}

fn swipe(id: i32, x: f64, y: f64) -> SwipeGestureEventArgs {
    SwipeGestureEventArgs::new(id, Vector::new(x, y), Vector::default())
}

fn completed_task() -> DispatcherTask<()> {
    Dispatcher::current_dispatcher().to_task_scheduler().start_local(async {})
}

/// The stand-in for a task completion source: its task completes when the
/// result is set.
#[derive(Default)]
struct TaskGate {
    is_set: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl TaskGate {
    fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    fn task(self: &Rc<Self>) -> DispatcherTask<()> {
        Dispatcher::current_dispatcher().to_task_scheduler().start_local(TaskGateFuture(self.clone()))
    }

    fn set_result(&self) {
        self.is_set.set(true);
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct TaskGateFuture(Rc<TaskGate>);

impl Future for TaskGateFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.is_set.get() {
            Poll::Ready(())
        } else {
            *self.0.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

struct StartCall {
    from: Option<Ref<Visual>>,
    to: Option<Ref<Visual>>,
    forward: bool,
    cancellation_token: CancellationToken,
}

type StartSetup = Box<dyn Fn(&StartCall) -> Option<DispatcherTask<()>>>;

/// A page transition that records how it was started.
#[derive(Default)]
struct MockTransition {
    starts: RefCell<Vec<StartCall>>,
    setup: RefCell<Option<StartSetup>>,
}

impl MockTransition {
    fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Sets up the task that a start returns; `None` leaves the default.
    fn setup(&self, setup: impl Fn(&StartCall) -> Option<DispatcherTask<()>> + 'static) {
        *self.setup.borrow_mut() = Some(Box::new(setup));
    }

    /// The number of starts that the predicate matches.
    fn count(&self, predicate: impl Fn(&StartCall) -> bool) -> usize {
        self.starts.borrow().iter().filter(|x| predicate(x)).count()
    }

    /// The number of starts between the two controls in the direction.
    fn count_of(&self, from: &Ref<Control>, to: &Ref<Control>, forward: bool) -> usize {
        self.count(|x| is_start(x, from, to, forward))
    }
}

fn is_start(call: &StartCall, from: &Ref<Control>, to: &Ref<Control>, forward: bool) -> bool {
    let from: Ref<Visual> = from.clone().upcast();
    let to: Ref<Visual> = to.clone().upcast();
    call.from.as_ref() == Some(&from) && call.to.as_ref() == Some(&to) && call.forward == forward
}

impl IPageTransition for MockTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let call = StartCall { from: from.cloned(), to: to.cloned(), forward, cancellation_token };
        let task = self.setup.borrow().as_ref().and_then(|setup| setup(&call));
        self.starts.borrow_mut().push(call);
        task.unwrap_or_else(completed_task)
    }
}

#[test]
fn initial_item_is_displayed() {
    let _app = start();
    let t = create_target(strs(&["foo", "bar"]));
    let target = &t.panel;

    assert_eq!(1, target.children().count());
    let container = content_presenter(&target.children().get(0));
    assert_eq!(content("foo"), content_of(&container));
}

#[test]
fn initial_selected_index_is_displayed() {
    let _app = start();
    let t = create_target_with(strs(&["foo", "bar"]), Options { selected_index: Some(1), ..Default::default() });
    let target = &t.panel;

    assert_eq!(1, target.children().count());
    let container = content_presenter(&target.children().get(0));
    assert_eq!(content("bar"), content_of(&container));
}

#[test]
fn refreshing_swipe_wiring_reuses_a_single_recognizer() {
    let _app = start();
    let t = create_target(strs(&["foo", "bar"]));
    let (target, carousel) = (&t.panel, &t.carousel);
    let page_slide = || -> Option<Rc<dyn IPageTransition>> {
        Some(Rc::new(PageSlide::with_duration(TimeSpan::from_milliseconds(1.0), SlideAxis::Horizontal)))
    };

    carousel.set_is_swipe_enabled(true);
    carousel.set_page_transition(page_slide());
    carousel.set_is_swipe_enabled(false);
    carousel.set_is_swipe_enabled(true);
    carousel.set_page_transition(None);
    carousel.set_page_transition(page_slide());

    let recognizers: Vec<Ref<SwipeGestureRecognizer>> = target
        .gesture_recognizers()
        .to_vec()
        .into_iter()
        .filter_map(|x| x.cast::<SwipeGestureRecognizer>())
        .collect();
    assert_eq!(1, recognizers.len());
    assert!(recognizers[0].is_enabled());
}

#[test]
fn displays_next_item() {
    let _app = start();
    let t = create_target(strs(&["foo", "bar"]));
    let (target, carousel) = (&t.panel, &t.carousel);

    carousel.set_selected_index(1);
    layout(target);

    assert_eq!(1, target.children().count());
    let container = content_presenter(&target.children().get(0));
    assert_eq!(content("bar"), content_of(&container));
}

#[test]
fn handles_inserted_item() {
    let _app = start();
    let items = string_list(&["foo", "bar"]);
    let t = create_target(items.clone());
    let target = &t.panel;
    let container = content_presenter(&target.children().get(0));

    items.insert(0, "baz".to_string());
    layout(target);

    assert_eq!(1, target.children().count());
    assert!(container.clone().upcast::<Control>() == target.children().get(0));
    assert_eq!(content("foo"), content_of(&container));
}

#[test]
fn handles_removed_item() {
    let _app = start();
    let items = string_list(&["foo", "bar"]);
    let t = create_target(items.clone());
    let target = &t.panel;
    let container = content_presenter(&target.children().get(0));

    items.remove_at(0);
    layout(target);

    assert_eq!(1, target.children().count());
    assert!(container.clone().upcast::<Control>() == target.children().get(0));
    assert_eq!(content("bar"), content_of(&container));
}

#[test]
fn handles_replaced_item() {
    let _app = start();
    let items = string_list(&["foo", "bar"]);
    let t = create_target(items.clone());
    let target = &t.panel;
    let container = content_presenter(&target.children().get(0));

    items.set(0, "baz".to_string());
    layout(target);

    assert_eq!(1, target.children().count());
    assert!(container.clone().upcast::<Control>() == target.children().get(0));
    assert_eq!(content("baz"), content_of(&container));
}

#[test]
fn handles_moved_item() {
    let _app = start();
    let items = string_list(&["foo", "bar"]);
    let t = create_target(items.clone());
    let target = &t.panel;
    let container = content_presenter(&target.children().get(0));

    items.move_item(0, 1);
    layout(target);

    assert_eq!(1, target.children().count());
    assert!(container.clone().upcast::<Control>() == target.children().get(0));
    assert_eq!(content("bar"), content_of(&container));
}

#[test]
fn handles_moved_item_range() {
    let _app = start();
    let items = string_list(&["foo", "bar", "baz", "qux", "quux"]);
    let t = create_target(items.clone());
    let (target, carousel) = (&t.panel, &t.carousel);
    let container = content_presenter(&target.children().get(0));

    carousel.set_selected_index(3);
    layout(target);
    items.move_range(0, 2, 4);
    layout(target);

    assert_eq!(1, target.children().count());
    assert!(container.clone().upcast::<Control>() == target.children().get(0));
    assert_eq!(content("qux"), content_of(&container));
    assert_eq!(1, carousel.selected_index());
}

#[test]
fn viewport_fraction_centers_selected_item_and_peeks_neighbors() {
    let _app = start();
    let t = create_target_with(
        strs(&["foo", "bar", "baz"]),
        Options { viewport_fraction: 0.8, client_size: Some(Size::new(400.0, 300.0)), ..Default::default() },
    );

    let realized_items = realized_by_content(&t.panel);

    assert_eq!(2, realized_items.len());
    assert_close(40.0, realized(&realized_items, "foo").bounds().x);
    assert_close(320.0, realized(&realized_items, "foo").bounds().width);
    assert_close(360.0, realized(&realized_items, "bar").bounds().x);
}

#[test]
fn viewport_fraction_one_third_shows_three_full_items() {
    let _app = start();
    let t = create_target_with(
        strs(&["foo", "bar", "baz", "qux"]),
        Options { viewport_fraction: 1.0 / 3.0, client_size: Some(Size::new(300.0, 120.0)), ..Default::default() },
    );
    let (target, carousel) = (&t.panel, &t.carousel);

    carousel.set_selected_index(1);
    layout(target);

    let realized_items = realized_by_content(target);

    assert_eq!(3, realized_items.len());
    assert_close(0.0, realized(&realized_items, "foo").bounds().x);
    assert_close(100.0, realized(&realized_items, "bar").bounds().x);
    assert_close(200.0, realized(&realized_items, "baz").bounds().x);
    assert_close(100.0, realized(&realized_items, "bar").bounds().width);
}

#[test]
fn changing_selected_index_repositions_fractional_viewport() {
    let _app = start();
    let t = create_target_with(
        strs(&["foo", "bar", "baz"]),
        Options { viewport_fraction: 0.8, client_size: Some(Size::new(400.0, 300.0)), ..Default::default() },
    );
    let (target, carousel) = (&t.panel, &t.carousel);

    carousel.set_selected_index(1);
    layout(target);

    let realized_items = realized_by_content(target);

    assert_close(40.0, realized(&realized_items, "bar").bounds().x);
    assert_close(-280.0, realized(&realized_items, "foo").bounds().x);
}

#[test]
fn changing_viewport_fraction_does_not_change_selected_item() {
    let _app = start();
    let t = create_target_with(
        strs(&["foo", "bar", "baz"]),
        Options { viewport_fraction: 0.72, client_size: Some(Size::new(400.0, 300.0)), ..Default::default() },
    );
    let (target, carousel) = (&t.panel, &t.carousel);

    carousel.set_wrap_selection(true);
    carousel.set_selected_index(2);
    layout(target);

    carousel.set_viewport_fraction(1.0);
    layout(target);

    let visible: Vec<_> = content_presenters(target).into_iter().filter(|x| x.is_visible()).collect();

    assert_eq!(1, visible.len());
    assert_eq!(content("baz"), content_of(&visible[0]));
    assert_eq!(2, carousel.selected_index());
}

mod transitions {
    use super::*;

    fn two_items() -> [Ref<Control>; 2] {
        [Button::new().upcast(), Canvas::new().upcast()]
    }

    fn three_items() -> [Ref<Control>; 3] {
        [Button::new().upcast(), Canvas::new().upcast(), Label::new().upcast()]
    }

    #[test]
    fn initial_item_does_not_start_transition() {
        let _app = start();
        let items = two_items();
        let transition = MockTransition::new();
        let _t = create_target_with_transition(controls_source(&items), transition.clone());

        assert_eq!(0, transition.count(|_| true));
    }

    #[test]
    fn changing_selected_index_starts_transition() {
        let _app = start();
        let items = two_items();
        let transition = MockTransition::new();
        let t = create_target_with_transition(controls_source(&items), transition.clone());

        t.carousel.set_selected_index(1);
        layout(&t.panel);

        assert_eq!(1, transition.count_of(&items[0], &items[1], true));
    }

    #[test]
    fn changing_selected_index_from_first_to_last_transitions_forward() {
        let _app = start();
        let items = three_items();
        let transition = MockTransition::new();
        let t = create_target_with_transition(controls_source(&items), transition.clone());

        t.carousel.set_selected_index(2);
        layout(&t.panel);

        run_jobs();

        assert_eq!(1, transition.count_of(&items[0], &items[2], true));
    }

    #[test]
    fn changing_selected_index_from_last_to_first_transitions_backward() {
        let _app = start();
        let items = three_items();
        let transition = MockTransition::new();
        let t = create_target_with_transition(controls_source(&items), transition.clone());

        t.carousel.set_selected_index(2);
        layout(&t.panel);
        run_jobs();

        t.carousel.set_selected_index(0);
        layout(&t.panel);
        run_jobs();

        assert_eq!(1, transition.count_of(&items[2], &items[0], false));
    }

    #[test]
    fn transition_from_control_is_recycled_when_transition_completes() {
        let _app = start();
        let items = two_items();
        let transition = MockTransition::new();
        let t = create_target_with_transition(controls_source(&items), transition.clone());
        let transition_task = TaskGate::new();

        {
            let (items, transition_task) = (items.clone(), transition_task.clone());
            transition
                .setup(move |x| is_start(x, &items[0], &items[1], true).then(|| transition_task.task()));
        }

        t.carousel.set_selected_index(1);
        layout(&t.panel);

        assert_eq!(items.to_vec(), t.panel.children().to_vec());
        assert!(items.iter().all(|x| x.is_visible()));

        transition_task.set_result();
        run_jobs();

        assert_eq!(items.to_vec(), t.panel.children().to_vec());
        assert!(!items[0].is_visible());
        assert!(items[1].is_visible());
    }

    #[test]
    fn existing_transition_is_canceled_if_interrupted() {
        let _app = start();
        let items = two_items();
        let transition = MockTransition::new();
        let t = create_target_with_transition(controls_source(&items), transition.clone());
        let transition_task = TaskGate::new();
        let cancelation_token: Rc<RefCell<Option<CancellationToken>>> = Rc::new(RefCell::new(None));

        {
            let (items, transition_task) = (items.clone(), transition_task.clone());
            let cancelation_token = cancelation_token.clone();
            transition.setup(move |x| {
                is_start(x, &items[0], &items[1], true).then(|| {
                    *cancelation_token.borrow_mut() = Some(x.cancellation_token.clone());
                    transition_task.task()
                })
            });
        }

        t.carousel.set_selected_index(1);
        layout(&t.panel);

        assert!(cancelation_token.borrow().is_some());
        assert!(!cancelation_token.borrow().as_ref().unwrap().is_cancellation_requested());

        t.carousel.set_selected_index(0);
        layout(&t.panel);

        assert!(cancelation_token.borrow().as_ref().unwrap().is_cancellation_requested());
    }

    #[test]
    fn completed_transition_is_flushed_before_starting_next_transition() {
        let _app = start();
        let items = three_items();
        let transition = MockTransition::new();

        transition.setup(|_| Some(completed_task()));

        let t = create_target_with_transition(controls_source(&items), transition.clone());

        t.carousel.set_selected_index(1);
        layout(&t.panel);

        t.carousel.set_selected_index(2);
        layout(&t.panel);

        assert_eq!(1, transition.count_of(&items[0], &items[1], true));
        assert_eq!(1, transition.count_of(&items[1], &items[2], true));

        run_jobs();
    }

    #[test]
    fn interrupted_transition_resets_current_page_before_starting_next_transition() {
        let _app = start();
        let items = three_items();
        let transition = Rc::new(DirtyStateTransition::default());
        let t = create_target_with_transition(controls_source(&items), transition.clone());

        t.carousel.set_selected_index(1);
        layout(&t.panel);

        t.carousel.set_selected_index(2);
        layout(&t.panel);

        let starts = transition.starts.borrow();
        assert_eq!(2, starts.len());
        assert_eq!(1.0, starts[1].0);
        assert!(starts[1].1.is_none());
    }

    #[derive(Default)]
    struct DirtyStateTransition {
        /// The opacity and the transform of the page that each start
        /// transitions from.
        starts: RefCell<Vec<(f64, Option<Rc<dyn ITransform>>)>>,
    }

    impl IPageTransition for DirtyStateTransition {
        fn start(
            &self,
            from: Option<&Ref<Visual>>,
            to: Option<&Ref<Visual>>,
            _forward: bool,
            _cancellation_token: CancellationToken,
        ) -> DispatcherTask<()> {
            self.starts
                .borrow_mut()
                .push((from.map_or(1.0, |from| from.opacity()), from.and_then(|from| from.render_transform())));

            if let Some(to) = to {
                to.set_opacity(0.25);
                let transform = TranslateTransform::new();
                transform.set_x(50.0);
                let transform: Rc<dyn ITransform> = (&transform).into();
                to.set_render_transform(Some(transform));
            }

            // Never completes: the reference waits until it is cancelled.
            Dispatcher::current_dispatcher().to_task_scheduler().start_local(std::future::pending::<()>())
        }
    }
}

mod wrap_selection_tests {
    use super::*;

    #[test]
    fn next_wraps_to_first_item_when_wrap_selection_enabled() {
        let _app = start();
        let t = create_target(strs(&["foo", "bar", "baz"]));
        let (target, carousel) = (&t.panel, &t.carousel);

        carousel.set_wrap_selection(true);
        carousel.set_selected_index(2); // Last item
        layout(target);

        carousel.next();
        layout(target);

        assert_eq!(0, carousel.selected_index());
    }

    #[test]
    fn next_does_not_wrap_when_wrap_selection_disabled() {
        let _app = start();
        let t = create_target(strs(&["foo", "bar", "baz"]));
        let (target, carousel) = (&t.panel, &t.carousel);

        carousel.set_wrap_selection(false);
        carousel.set_selected_index(2); // Last item
        layout(target);

        carousel.next();
        layout(target);

        assert_eq!(2, carousel.selected_index()); // Should stay at last item
    }

    #[test]
    fn previous_wraps_to_last_item_when_wrap_selection_enabled() {
        let _app = start();
        let t = create_target(strs(&["foo", "bar", "baz"]));
        let (target, carousel) = (&t.panel, &t.carousel);

        carousel.set_wrap_selection(true);
        carousel.set_selected_index(0); // First item
        layout(target);

        carousel.previous();
        layout(target);

        assert_eq!(2, carousel.selected_index()); // Should wrap to last item
    }

    #[test]
    fn previous_does_not_wrap_when_wrap_selection_disabled() {
        let _app = start();
        let t = create_target(strs(&["foo", "bar", "baz"]));
        let (target, carousel) = (&t.panel, &t.carousel);

        carousel.set_wrap_selection(false);
        carousel.set_selected_index(0); // First item
        layout(target);

        carousel.previous();
        layout(target);

        assert_eq!(0, carousel.selected_index()); // Should stay at first item
    }

    #[test]
    fn wrap_selection_works_with_two_items() {
        let _app = start();
        let t = create_target(strs(&["foo", "bar"]));
        let (target, carousel) = (&t.panel, &t.carousel);

        carousel.set_wrap_selection(true);
        carousel.set_selected_index(1);
        layout(target);

        carousel.next();
        layout(target);

        assert_eq!(0, carousel.selected_index());

        carousel.previous();
        layout(target);

        assert_eq!(1, carousel.selected_index());
    }

    #[test]
    fn wrap_selection_does_not_apply_to_single_item() {
        let _app = start();
        let t = create_target(strs(&["foo"]));
        let (target, carousel) = (&t.panel, &t.carousel);

        carousel.set_wrap_selection(true);
        carousel.set_selected_index(0);
        layout(target);

        carousel.next();
        layout(target);

        assert_eq!(0, carousel.selected_index());

        carousel.previous();
        layout(target);

        assert_eq!(0, carousel.selected_index());
    }
}

mod gestures {
    use super::*;

    fn reset(visual: &Ref<Visual>) {
        visual.set_render_transform(None);
        visual.set_opacity(1.0);
        visual.set_z_index(0);
        visual.set_value(Visual::clip_property(), None);
    }

    fn translate(x: f64, y: f64) -> Option<Rc<dyn ITransform>> {
        let transform: Rc<dyn ITransform> = (&TranslateTransform::with_offset(x, y)).into();
        Some(transform)
    }

    #[test]
    fn swiping_forward_realizes_next_item() {
        let _app = start();
        let t = create_target(strs(&["foo", "bar"]));
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);

        let e = swipe(1, 10.0, 0.0);
        panel.raise_event(&e);

        assert!(carousel.is_swiping());
        assert_eq!(2, panel.children().count());
        let target = panel.children().get(1);
        assert!(target.is_visible());
        assert_eq!(content("bar"), target.cast::<ContentPresenter>().and_then(|x| content_of(&x)));
    }

    #[test]
    fn swiping_backward_at_start_rubber_bands_when_wrap_selection_false() {
        let _app = start();
        let t = create_target(strs(&["foo", "bar"]));
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);
        carousel.set_wrap_selection(false);

        let e = swipe(1, -10.0, 0.0);
        panel.raise_event(&e);

        assert!(carousel.is_swiping());
        assert_eq!(1, panel.children().count());
    }

    #[test]
    fn swiping_backward_at_start_wraps_when_wrap_selection_true() {
        let _app = start();
        let t = create_target(strs(&["foo", "bar", "baz"]));
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);
        carousel.set_wrap_selection(true);

        let e = swipe(1, -10.0, 0.0);
        panel.raise_event(&e);

        assert!(carousel.is_swiping());
        assert_eq!(2, panel.children().count());
        let target = panel.children().get(1);
        assert_eq!(content("baz"), target.cast::<ContentPresenter>().and_then(|x| content_of(&x)));
    }

    #[test]
    fn viewport_fraction_swiping_backward_at_start_wraps_when_wrap_selection_true() {
        let (_app, clock) = start_with_clock();

        let t = create_target_with(
            strs(&["foo", "bar", "baz"]),
            Options { viewport_fraction: 0.8, ..Default::default() },
        );
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);
        carousel.set_wrap_selection(true);
        layout(panel);

        panel.raise_event(&swipe(1, -120.0, 0.0));

        assert!(carousel.is_swiping());
        assert!(content_presenters(panel).iter().any(|x| content_of(x) == content("baz")));

        panel.raise_event(&SwipeGestureEndedEventArgs::new(1, Vector::default()));

        clock.pulse(TimeSpan::ZERO);
        clock.pulse(TimeSpan::from_seconds(1.0));
        run_jobs();

        assert_eq!(2, carousel.selected_index());
    }

    #[test]
    fn swiping_forward_at_end_rubber_bands_when_wrap_selection_false() {
        let _app = start();
        let t = create_target(strs(&["foo", "bar"]));
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);
        carousel.set_wrap_selection(false);
        carousel.set_selected_index(1);

        layout(panel);
        layout(panel);

        assert_eq!(Some(2), carousel.items_source().map(|x| x.count()));
        assert_eq!(1, carousel.selected_index());
        assert!(!carousel.wrap_selection(), "WrapSelection should be false");

        let container = content_presenter(&panel.children().get(0));
        assert_eq!(content("bar"), content_of(&container));

        let e = swipe(1, 10.0, 0.0);
        panel.raise_event(&e);

        assert!(carousel.is_swiping());
        assert_eq!(1, panel.children().count());
    }

    #[test]
    fn swiping_locks_to_dominant_axis() {
        let _app = start();
        let t = create_target_with_transition(
            strs(&["foo", "bar"]),
            Rc::new(CrossFade::with_duration(TimeSpan::from_seconds(1.0))),
        );
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);

        let e = swipe(1, 10.0, 2.0);
        panel.raise_event(&e);

        assert!(carousel.is_swiping());
    }

    #[test]
    fn swipe_completion_does_not_update_with_same_from_and_to() {
        let (_app, clock) = start_with_clock();

        let transition = Rc::new(TrackingInteractiveTransition::default());
        let t = create_target_with_transition(strs(&["foo", "bar"]), transition.clone());
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);

        panel.raise_event(&swipe(1, 1000.0, 0.0));
        panel.raise_event(&SwipeGestureEndedEventArgs::new(1, Vector::new(1000.0, 0.0)));

        clock.pulse(TimeSpan::ZERO);
        clock.pulse(TimeSpan::from_seconds(1.0));
        run_jobs();

        assert!(transition.update_call_count.get() > 0);
        assert!(!transition.saw_aliased_update.get());
        assert_eq!(1.0, transition.last_progress.get());
        assert_eq!(1, carousel.selected_index());
    }

    #[test]
    fn swipe_completion_keeps_target_final_interactive_visual_state() {
        let (_app, clock) = start_with_clock();

        let transition = Rc::new(TransformTrackingInteractiveTransition::default());
        let t = create_target_with_transition(strs(&["foo", "bar"]), transition.clone());
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);

        panel.raise_event(&swipe(1, 1000.0, 0.0));
        panel.raise_event(&SwipeGestureEndedEventArgs::new(1, Vector::new(1000.0, 0.0)));

        clock.pulse(TimeSpan::ZERO);
        clock.pulse(TimeSpan::from_seconds(1.0));
        run_jobs();

        assert_eq!(1, carousel.selected_index());
        let matching: Vec<_> =
            content_presenters(panel).into_iter().filter(|x| content_of(x) == content("bar")).collect();
        assert_eq!(1, matching.len());
        let realized = &matching[0];
        let last_target_transform = transition.last_target_transform.borrow().clone();
        assert!(last_target_transform.is_some());
        let last_target_transform: Rc<dyn ITransform> = (&last_target_transform.unwrap()).into();
        assert!(realized.render_transform().is_some_and(|x| *x == *last_target_transform));
    }

    #[test]
    fn swipe_completion_hides_outgoing_page_before_resetting_visual_state() {
        let (_app, clock) = start_with_clock();

        let transition = Rc::new(OutgoingTransformTrackingInteractiveTransition);
        let t = create_target_with_transition(strs(&["foo", "bar"]), transition);
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);

        let matching: Vec<_> =
            content_presenters(panel).into_iter().filter(|x| content_of(x) == content("foo")).collect();
        assert_eq!(1, matching.len());
        let outgoing = matching[0].clone();
        let hidden_when_reset: Rc<Cell<Option<bool>>> = Rc::new(Cell::new(None));
        let _subscription = {
            let hidden_when_reset = hidden_when_reset.clone();
            let outgoing_weak = outgoing.downgrade();
            outgoing.property_changed(move |args| {
                if args.property() == Visual::render_transform_property().as_property()
                    && args.get_new_value::<Option<Rc<dyn ITransform>>>().is_none()
                {
                    if let Some(outgoing) = outgoing_weak.upgrade() {
                        hidden_when_reset.set(Some(!outgoing.is_visible()));
                    }
                }
            })
        };

        panel.raise_event(&swipe(1, 1000.0, 0.0));
        panel.raise_event(&SwipeGestureEndedEventArgs::new(1, Vector::new(1000.0, 0.0)));

        clock.pulse(TimeSpan::ZERO);
        clock.pulse(TimeSpan::from_seconds(1.0));
        run_jobs();

        assert_eq!(Some(true), hidden_when_reset.get());
    }

    #[test]
    fn rubber_band_swipe_release_animates_back_through_intermediate_progress() {
        let (_app, clock) = start_with_clock();

        let transition = Rc::new(ProgressTrackingInteractiveTransition::default());
        let t = create_target_with_transition(strs(&["foo", "bar"]), transition.clone());
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);
        carousel.set_wrap_selection(false);

        panel.raise_event(&swipe(1, -100.0, 0.0));

        let release_start_progress = *transition.progresses.borrow().last().unwrap();
        let updates_before_release = transition.progresses.borrow().len();

        panel.raise_event(&SwipeGestureEndedEventArgs::new(1, Vector::default()));

        clock.pulse(TimeSpan::ZERO);
        clock.pulse(TimeSpan::from_seconds(0.1));
        run_jobs();

        let post_release_progresses: Vec<f64> =
            transition.progresses.borrow().iter().skip(updates_before_release).copied().collect();

        assert!(post_release_progresses.iter().any(|&p| p > 0.0 && p < release_start_progress));

        clock.pulse(TimeSpan::from_seconds(1.0));
        run_jobs();

        assert_eq!(0.0, *transition.progresses.borrow().last().unwrap());
        assert_eq!(0, carousel.selected_index());
    }

    #[test]
    fn viewport_fraction_selected_index_change_drives_progress_updates() {
        let (_app, clock) = start_with_clock();

        let transition = Rc::new(ProgressTrackingInteractiveTransition::default());
        let t = create_target_with(
            strs(&["foo", "bar", "baz"]),
            Options { transition: Some(transition.clone()), viewport_fraction: 0.8, ..Default::default() },
        );
        let carousel = &t.carousel;

        carousel.set_selected_index(1);

        clock.pulse(TimeSpan::ZERO);
        clock.pulse(TimeSpan::from_seconds(0.1));
        clock.pulse(TimeSpan::from_seconds(1.0));
        run_jobs();

        let progresses = transition.progresses.borrow();
        assert!(!progresses.is_empty());
        assert!(progresses.iter().any(|&p| p > 0.0 && p < 1.0));
        assert_eq!(1.0, *progresses.last().unwrap());
        assert_eq!(1, carousel.selected_index());
    }

    #[derive(Default)]
    struct TrackingInteractiveTransition {
        update_call_count: Cell<i32>,
        saw_aliased_update: Cell<bool>,
        last_progress: Cell<f64>,
    }

    impl IPageTransition for TrackingInteractiveTransition {
        fn start(
            &self,
            _from: Option<&Ref<Visual>>,
            _to: Option<&Ref<Visual>>,
            _forward: bool,
            _cancellation_token: CancellationToken,
        ) -> DispatcherTask<()> {
            completed_task()
        }

        fn as_progress_page_transition(&self) -> Option<&dyn IProgressPageTransition> {
            Some(self)
        }
    }

    impl IProgressPageTransition for TrackingInteractiveTransition {
        fn update(
            &self,
            progress: f64,
            from: Option<&Ref<Visual>>,
            to: Option<&Ref<Visual>>,
            _forward: bool,
            _page_length: f64,
            _visible_items: &[PageTransitionItem],
        ) {
            self.update_call_count.set(self.update_call_count.get() + 1);
            self.last_progress.set(progress);

            if from.is_some() && from == to {
                self.saw_aliased_update.set(true);
            }
        }

        fn reset(&self, visual: &Ref<Visual>) {
            reset(visual);
        }
    }

    #[derive(Default)]
    struct ProgressTrackingInteractiveTransition {
        progresses: RefCell<Vec<f64>>,
    }

    impl IPageTransition for ProgressTrackingInteractiveTransition {
        fn start(
            &self,
            _from: Option<&Ref<Visual>>,
            _to: Option<&Ref<Visual>>,
            _forward: bool,
            _cancellation_token: CancellationToken,
        ) -> DispatcherTask<()> {
            completed_task()
        }

        fn as_progress_page_transition(&self) -> Option<&dyn IProgressPageTransition> {
            Some(self)
        }
    }

    impl IProgressPageTransition for ProgressTrackingInteractiveTransition {
        fn update(
            &self,
            progress: f64,
            _from: Option<&Ref<Visual>>,
            _to: Option<&Ref<Visual>>,
            _forward: bool,
            _page_length: f64,
            _visible_items: &[PageTransitionItem],
        ) {
            self.progresses.borrow_mut().push(progress);
        }

        fn reset(&self, visual: &Ref<Visual>) {
            reset(visual);
        }
    }

    #[derive(Default)]
    struct TransformTrackingInteractiveTransition {
        last_target_transform: RefCell<Option<Ref<TransformGroup>>>,
    }

    impl IPageTransition for TransformTrackingInteractiveTransition {
        fn start(
            &self,
            _from: Option<&Ref<Visual>>,
            _to: Option<&Ref<Visual>>,
            _forward: bool,
            _cancellation_token: CancellationToken,
        ) -> DispatcherTask<()> {
            completed_task()
        }

        fn as_progress_page_transition(&self) -> Option<&dyn IProgressPageTransition> {
            Some(self)
        }
    }

    impl IProgressPageTransition for TransformTrackingInteractiveTransition {
        fn update(
            &self,
            progress: f64,
            _from: Option<&Ref<Visual>>,
            to: Option<&Ref<Visual>>,
            _forward: bool,
            _page_length: f64,
            _visible_items: &[PageTransitionItem],
        ) {
            let Some(target) = to.and_then(|to| to.clone().cast::<Control>()) else {
                return;
            };

            let existing = target.render_transform().and_then(|transform| {
                transform.as_object().and_then(|object| object.downcast_ref::<TransformGroup>()).map(|x| x.to_ref())
            });
            let group = match existing {
                Some(group) => group,
                None => {
                    let group = TransformGroup::new();
                    group.children().add(ScaleTransform::new().upcast::<Transform>());
                    group.children().add(TranslateTransform::new().upcast::<Transform>());
                    let transform: Rc<dyn ITransform> = (&group).into();
                    target.set_render_transform(Some(transform));
                    group
                }
            };

            let scale = group.children().get(0).cast::<ScaleTransform>().expect("Expected a scale transform.");
            let translate =
                group.children().get(1).cast::<TranslateTransform>().expect("Expected a translate transform.");
            scale.set_scale_y(0.9 + (0.1 * progress));
            scale.set_scale_x(scale.scale_y());
            translate.set_x(100.0 * (1.0 - progress));
            *self.last_target_transform.borrow_mut() = Some(group);
        }

        fn reset(&self, visual: &Ref<Visual>) {
            visual.set_render_transform(None);
        }
    }

    struct OutgoingTransformTrackingInteractiveTransition;

    impl IPageTransition for OutgoingTransformTrackingInteractiveTransition {
        fn start(
            &self,
            _from: Option<&Ref<Visual>>,
            _to: Option<&Ref<Visual>>,
            _forward: bool,
            _cancellation_token: CancellationToken,
        ) -> DispatcherTask<()> {
            completed_task()
        }

        fn as_progress_page_transition(&self) -> Option<&dyn IProgressPageTransition> {
            Some(self)
        }
    }

    impl IProgressPageTransition for OutgoingTransformTrackingInteractiveTransition {
        fn update(
            &self,
            progress: f64,
            from: Option<&Ref<Visual>>,
            to: Option<&Ref<Visual>>,
            _forward: bool,
            _page_length: f64,
            _visible_items: &[PageTransitionItem],
        ) {
            if let Some(source) = from.filter(|from| from.is::<Control>()) {
                source.set_render_transform(translate(100.0 * progress, 0.0));
            }

            if let Some(target) = to.filter(|to| to.is::<Control>()) {
                target.set_render_transform(translate(100.0 * (1.0 - progress), 0.0));
            }
        }

        fn reset(&self, visual: &Ref<Visual>) {
            visual.set_render_transform(None);
        }
    }

    #[test]
    fn vertical_swipe_forward_realizes_next_item() {
        let _app = start();
        let transition = Rc::new(PageSlide::with_duration(TimeSpan::from_seconds(1.0), SlideAxis::Vertical));
        let t = create_target_with_transition(strs(&["foo", "bar"]), transition);
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);

        let e = swipe(1, 0.0, 10.0);
        panel.raise_event(&e);

        assert!(carousel.is_swiping());
        assert_eq!(2, panel.children().count());
        let target = panel.children().get(1).cast::<ContentPresenter>();
        assert!(target.is_some());
        assert_eq!(content("bar"), content_of(&target.unwrap()));
    }

    #[test]
    fn new_swipe_interrupts_active_completion_animation() {
        let (_app, clock) = start_with_clock();

        let transition = Rc::new(TrackingInteractiveTransition::default());
        let t = create_target_with_transition(strs(&["foo", "bar", "baz"]), transition);
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);

        panel.raise_event(&swipe(1, 1000.0, 0.0));
        panel.raise_event(&SwipeGestureEndedEventArgs::new(1, Vector::new(1000.0, 0.0)));

        clock.pulse(TimeSpan::ZERO);
        clock.pulse(TimeSpan::from_milliseconds(50.0));
        run_jobs();

        assert_eq!(0, carousel.selected_index());

        panel.raise_event(&swipe(2, 10.0, 0.0));

        assert!(carousel.is_swiping());
        assert_eq!(1, carousel.selected_index());
    }

    #[test]
    fn swipe_with_non_interactive_transition_does_not_crash() {
        let _app = start();
        let transition = MockTransition::new();
        transition.setup(|_| Some(completed_task()));
        let t = create_target_with_transition(strs(&["foo", "bar"]), transition);
        let (panel, carousel) = (&t.panel, &t.carousel);
        carousel.set_is_swipe_enabled(true);

        let e = swipe(1, 10.0, 0.0);
        panel.raise_event(&e);

        assert!(carousel.is_swiping());
        assert_eq!(2, panel.children().count());
    }
}
