//! The reference tests run the continuations of asynchronous methods by
//! executing the callbacks posted to a test synchronization context; here
//! continuations are jobs of the dispatcher of the test, so running its jobs
//! stands for that step.
//!
//! The test transition of the reference is an asynchronous method whose
//! continuation runs inline when its completion source is set; here the
//! bookkeeping of that continuation (clearing the completion source and
//! counting) is done where the source is set, and the returned task completes
//! from a job of the dispatcher.

use crate::presenters::ContentPresenter;
use crate::templates::{FuncControlTemplate, IControlTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot, TestScope};
use crate::{
    Button, Canvas, Control, Panel, TransitionCompletedEventArgs, TransitioningContentControl,
};
use ferroui_base::animation::IPageTransition;
use ferroui_base::threading::{CancellationToken, Dispatcher, DispatcherTask};
use ferroui_base::{BoxedValue, Ref, StaticType, StyledElement, Visual};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

fn start() -> TestScope {
    test_scope()
}

/// Runs the continuations of the asynchronous methods.
fn execute_posted_callbacks() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn create_target(content: Option<BoxedValue>) -> (Ref<TransitioningContentControl>, Rc<TestTransition>, Ref<TestRoot>) {
    let transition = TestTransition::new();
    let target = TransitioningContentControl::new();
    target.set_content(content);
    let page_transition: Rc<dyn IPageTransition> = transition.clone();
    target.set_page_transition(Some(page_transition));
    target.set_template(Some(create_template()));

    let root = TestRoot::with_child(target.clone());
    root.execute_initial_layout_pass();
    (target, transition, root)
}

fn create_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, _| {
        let panel = Panel::new();

        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        panel.children().add(presenter);

        let presenter2 = ContentPresenter::new();
        presenter2.set_name(Some("PART_ContentPresenter2".to_string()));
        panel.children().add(presenter2);

        panel.upcast()
    })
}

fn get_content_presenters2(target: &TransitioningContentControl) -> Ref<ContentPresenter> {
    let presenter = target
        .get_template_descendants()
        .into_iter()
        .filter_map(|x| x.cast::<Control>())
        .find(|x| x.name().as_deref() == Some("PART_ContentPresenter2"))
        .expect("no second content presenter");
    assert!(std::ptr::eq(presenter.get_type(), <ContentPresenter as StaticType>::TYPE));
    presenter.cast::<ContentPresenter>().unwrap()
}

fn layout(c: &Control) {
    if let Some(layout_manager) = c.get_layout_manager() {
        layout_manager.execute_layout_pass();
    }
}

fn content_of(presenter: &ContentPresenter) -> Option<String> {
    presenter.content().and_then(|content| string_of(&content))
}

fn text(value: &str) -> Option<String> {
    Some(value.to_string())
}

thread_local! {
    /// The actions that run when a cancellation token of a test transition
    /// is cancelled; a token only carries the key of its action.
    static CANCEL_ACTIONS: RefCell<HashMap<u64, Box<dyn FnOnce()>>> = RefCell::new(HashMap::new());
    static NEXT_CANCEL_ACTION: Cell<u64> = const { Cell::new(0) };
}

fn register_cancel_action(token: &CancellationToken, action: impl FnOnce() + 'static) {
    let id = NEXT_CANCEL_ACTION.with(|next| next.replace(next.get() + 1));
    CANCEL_ACTIONS.with(|actions| actions.borrow_mut().insert(id, Box::new(action)));
    let _ = token.register(move || {
        let action = CANCEL_ACTIONS.with(|actions| actions.borrow_mut().remove(&id));
        if let Some(action) = action {
            action();
        }
    });
}

/// The stand-in for a task completion source: its task completes when the
/// result is set.
#[derive(Default)]
struct TaskGate {
    is_set: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl TaskGate {
    fn task(self: &Rc<Self>) -> DispatcherTask<()> {
        Dispatcher::current_dispatcher().to_task_scheduler().start_local(TaskGateFuture(self.clone()))
    }

    /// Sets the result; false if it was set already.
    fn try_set_result(&self) -> bool {
        if self.is_set.replace(true) {
            return false;
        }
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
        true
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

type StartedHandler = Box<dyn Fn(Option<&Ref<Visual>>, Option<&Ref<Visual>>, bool)>;

#[derive(Default)]
struct TestTransitionState {
    /// The completion source of the running transition and its token.
    tcs: RefCell<Option<(Rc<TaskGate>, CancellationToken)>>,
    start_count: Cell<i32>,
    finish_count: Cell<i32>,
    cancel_count: Cell<i32>,
    started: RefCell<Vec<Rc<StartedHandler>>>,
}

impl TestTransitionState {
    /// Sets the result of the running transition and runs what follows its
    /// await.
    fn try_set_result(&self) {
        let Some((gate, token)) = self.tcs.borrow().clone() else {
            return;
        };
        if !gate.try_set_result() {
            return;
        }
        *self.tcs.borrow_mut() = None;

        if !token.is_cancellation_requested() {
            self.finish_count.set(self.finish_count.get() + 1);
        } else {
            self.cancel_count.set(self.cancel_count.get() + 1);
        }
    }
}

struct TestTransition {
    state: Rc<TestTransitionState>,
}

impl TestTransition {
    fn new() -> Rc<Self> {
        Rc::new(Self { state: Rc::new(TestTransitionState::default()) })
    }

    fn start_count(&self) -> i32 {
        self.state.start_count.get()
    }

    fn cancel_count(&self) -> i32 {
        self.state.cancel_count.get()
    }

    fn started(&self, handler: impl Fn(Option<&Ref<Visual>>, Option<&Ref<Visual>>, bool) + 'static) {
        self.state.started.borrow_mut().push(Rc::new(Box::new(handler)));
    }

    fn complete(&self) {
        assert!(self.state.tcs.borrow().is_some(), "no transition is running");
        self.state.try_set_result();
    }
}

impl IPageTransition for TestTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let state = &self.state;
        state.start_count.set(state.start_count.get() + 1);
        let started = state.started.borrow().clone();
        for handler in started {
            handler(from, to, forward);
        }
        if state.tcs.borrow().is_some() {
            panic!("Transition already running");
        }
        let gate = Rc::new(TaskGate::default());
        *state.tcs.borrow_mut() = Some((gate.clone(), cancellation_token.clone()));
        register_cancel_action(&cancellation_token, {
            let state = state.clone();
            move || state.try_set_result()
        });
        gate.task()
    }
}

/// What a completed transition reported: the contents and whether it ran to
/// completion.
type Completed = (Option<String>, Option<String>, bool);

fn record_completed_transitions(target: &TransitioningContentControl) -> Rc<RefCell<Vec<Completed>>> {
    let completed_transitions = Rc::new(RefCell::new(Vec::new()));
    target.transition_completed({
        let completed_transitions = completed_transitions.clone();
        move |_, e: &TransitionCompletedEventArgs| {
            completed_transitions.borrow_mut().push((
                e.from().and_then(string_of),
                e.to().and_then(string_of),
                e.has_run_to_completion(),
            ));
        }
    });
    completed_transitions
}

fn completed(from: &str, to: &str, has_run_to_completion: bool) -> Completed {
    (text(from), text(to), has_run_to_completion)
}

#[test]
fn transition_should_not_be_run_when_first_shown() {
    let _app = start();
    let (_target, transition, _root) = create_target(boxed_str("foo"));

    assert_eq!(0, transition.start_count());
}

#[test]
fn content_presenters2_should_initially_be_hidden() {
    let _app = start();
    let (target, _transition, _root) = create_target(boxed_str("foo"));
    let presenter2 = get_content_presenters2(&target);

    assert!(!presenter2.is_visible());
}

#[test]
fn transition_should_be_run_on_layout() {
    let _app = start();
    let (target, transition, _root) = create_target(boxed_str("foo"));

    target.set_content(boxed_str("bar"));
    assert_eq!(0, transition.start_count());

    layout(&target);
    assert_eq!(1, transition.start_count());
}

#[test]
fn control_transition_should_be_run_on_layout() {
    let _app = start();
    let (target, transition, _root) = create_target(Some(Control::boxed(Button::new())));

    target.set_content(Some(Control::boxed(Canvas::new())));
    assert_eq!(0, transition.start_count());

    layout(&target);
    assert_eq!(1, transition.start_count());
}

#[test]
fn control_should_connect_to_visual_tree_once() {
    let _app = start();
    let (target, _transition, _root) = create_target(Some(Control::boxed(Control::new())));

    let control = Control::new();
    let counter = Rc::new(Cell::new(0));

    control.attached_to_visual_tree({
        let counter = counter.clone();
        move |_| counter.set(counter.get() + 1)
    });

    target.set_content(Some(Control::boxed(control)));
    layout(&target);
    target.set_content(Some(Control::boxed(Control::new())));
    layout(&target);

    assert_eq!(1, counter.get());
}

#[test]
fn content_presenters2_should_be_setup() {
    let _app = start();
    let (target, _transition, _root) = create_target(boxed_str("foo"));
    let presenter1 = target.presenter().unwrap();
    let presenter2 = get_content_presenters2(&target);

    target.set_content(boxed_str("bar"));
    layout(&target);

    assert!(presenter2.is_visible());
    assert_eq!(text("foo"), content_of(&presenter1));
    assert_eq!(text("bar"), content_of(&presenter2));
}

#[test]
fn old_presenter_should_be_hidden_when_transition_completes() {
    let _app = start();
    let (target, transition, _root) = create_target(boxed_str("foo"));
    let presenter1 = target.presenter().unwrap();
    let presenter2 = get_content_presenters2(&target);

    target.set_content(boxed_str("bar"));
    layout(&target);
    assert!(presenter1.is_visible());
    assert!(presenter2.is_visible());

    transition.complete();
    execute_posted_callbacks();
    assert!(presenter2.is_visible());
    assert!(!presenter1.is_visible());

    target.set_content(boxed_str("foo"));
    layout(&target);
    assert!(presenter1.is_visible());
    assert!(presenter2.is_visible());

    transition.complete();
    execute_posted_callbacks();
    assert!(presenter1.is_visible());
    assert!(!presenter2.is_visible());
}

#[test]
fn transition_completed_should_be_raised_when_content_changes() {
    let _app = start();
    let (target, transition, _root) = create_target(boxed_str("foo"));

    let completed_transitions = record_completed_transitions(&target);

    target.set_content(boxed_str("bar"));
    layout(&target);
    assert_eq!(Vec::<Completed>::new(), *completed_transitions.borrow());

    transition.complete();
    execute_posted_callbacks();
    assert_eq!(vec![completed("foo", "bar", true)], *completed_transitions.borrow());

    target.set_content(boxed_str("foo"));
    layout(&target);
    assert_eq!(vec![completed("foo", "bar", true)], *completed_transitions.borrow());

    transition.complete();
    execute_posted_callbacks();
    assert_eq!(
        vec![completed("foo", "bar", true), completed("bar", "foo", true)],
        *completed_transitions.borrow()
    );
}

#[test]
fn transition_should_be_canceled_if_content_changes_while_running() {
    let _app = start();
    let (target, transition, _root) = create_target(boxed_str("foo"));

    target.set_content(boxed_str("bar"));
    layout(&target);
    target.set_content(boxed_str("baz"));

    assert_eq!(0, transition.cancel_count());

    layout(&target);

    assert_eq!(1, transition.cancel_count());
}

#[test]
fn new_transition_should_be_started_if_content_changes_while_running() {
    let _app = start();
    let (target, transition, _root) = create_target(boxed_str("foo"));
    let presenter2 = get_content_presenters2(&target);

    target.set_content(boxed_str("bar"));
    layout(&target);

    target.set_content(boxed_str("baz"));

    let started_raised = Rc::new(Cell::new(0));

    transition.started({
        let started_raised = started_raised.clone();
        let presenter2 = presenter2.clone();
        let target = target.clone();
        let state = transition.state.clone();
        move |from, to, forward| {
            let content_presenter = |visual: Option<&Ref<Visual>>| {
                let visual = visual.expect("no visual");
                assert!(std::ptr::eq(visual.get_type(), <ContentPresenter as StaticType>::TYPE));
                visual.clone().cast::<ContentPresenter>().unwrap()
            };
            let from_presenter = content_presenter(from);
            let to_presenter = content_presenter(to);

            assert!(presenter2.ptr_eq(&from_presenter));
            assert!(target.presenter().unwrap().ptr_eq(&to_presenter));
            assert_eq!(text("bar"), content_of(&from_presenter));
            assert_eq!(text("baz"), content_of(&to_presenter));
            assert!(forward);
            assert_eq!(1, state.cancel_count.get());

            started_raised.set(started_raised.get() + 1);
        }
    });

    layout(&target);
    execute_posted_callbacks();

    assert_eq!(1, started_raised.get());
    assert_eq!(text("baz"), content_of(&target.presenter().unwrap()));
    assert_eq!(text("bar"), content_of(&presenter2));
}

#[test]
fn transition_completed_should_be_raised_if_content_changes_while_running() {
    let _app = start();
    let (target, _transition, _root) = create_target(boxed_str("foo"));

    let completed_transitions = record_completed_transitions(&target);

    target.set_content(boxed_str("bar"));
    layout(&target);
    execute_posted_callbacks();
    assert_eq!(Vec::<Completed>::new(), *completed_transitions.borrow());

    target.set_content(boxed_str("baz"));
    layout(&target);
    execute_posted_callbacks();
    assert_eq!(vec![completed("foo", "bar", false)], *completed_transitions.borrow());
}

#[test]
fn transition_should_be_reversed_if_property_is_set() {
    for reversed in [false, true] {
        let _app = start();
        let (target, transition, _root) = create_target(boxed_str("foo"));
        let presenter2 = get_content_presenters2(&target);

        target.set_is_transition_reversed(reversed);

        target.set_content(boxed_str("bar"));

        let started_raised = Rc::new(Cell::new(0));

        transition.started({
            let started_raised = started_raised.clone();
            move |_, _, forward| {
                assert_eq!(reversed, !forward);

                started_raised.set(started_raised.get() + 1);
            }
        });

        layout(&target);
        execute_posted_callbacks();

        assert_eq!(1, started_raised.get());
        assert_eq!(text("foo"), content_of(&target.presenter().unwrap()));
        assert_eq!(text("bar"), content_of(&presenter2));
    }
}

#[test]
fn logical_children_should_not_be_duplicated() {
    let _app = start();
    let (target, _transition, _root) = create_target(boxed_str(""));
    target.set_page_transition(None);

    let child_control = Control::new();
    target.set_content(Some(Control::boxed(child_control.clone())));

    let logical_children = StyledElement::logical_children(&target);
    assert_eq!(1, logical_children.count());
    assert_eq!(logical_children.get(0), child_control.upcast::<StyledElement>());
}

#[test]
fn first_presenter_should_register_tcc_as_his_host() {
    let _app = start();
    let (target, _transition, _root) = create_target(boxed_str(""));
    target.set_page_transition(None);

    let child_control = Control::new();
    target.presenter().unwrap().set_content(Some(Control::boxed(child_control.clone())));

    let logical_children = StyledElement::logical_children(&target);
    assert_eq!(1, logical_children.count());
    assert_eq!(logical_children.get(0), child_control.upcast::<StyledElement>());
}

#[test]
fn old_content_should_be_null_when_new_content_is_old_one() {
    let _app = start();
    let (target, _transition, _root) = create_target(boxed_str(""));
    let presenter2 = get_content_presenters2(&target);
    target.set_page_transition(None);

    let child_control = Control::new();
    target.presenter().unwrap().set_content(Some(Control::boxed(child_control)));

    const FAKE_PAGE1: &str = "fakePage1";
    const FAKE_PAGE2: &str = "fakePage2";

    target.presenter().unwrap().set_content(boxed_str(FAKE_PAGE1));
    target.presenter().unwrap().set_content(boxed_str(FAKE_PAGE2));
    target.presenter().unwrap().set_content(boxed_str(FAKE_PAGE1));

    assert_eq!(text(FAKE_PAGE1), content_of(&target.presenter().unwrap()));
    assert!(presenter2.content().is_none());
}

// ---------------------------------------------------------------------------
// Additional tests of this port (not present in the reference test file):
// the content of the last presenter is compared with the new content by
// reference.
// ---------------------------------------------------------------------------

/// Re-attaches the target (which shows its content again without a
/// transition) and returns whether the first presenter still had content at
/// the moment the content of the second presenter was set.
fn first_presenter_has_content_when_second_is_set(
    target: &Ref<TransitioningContentControl>,
    root: &Ref<TestRoot>,
) -> bool {
    let presenter1 = target.presenter().unwrap();
    let presenter2 = get_content_presenters2(target);
    let observed = Rc::new(Cell::new(None));

    let subscription = presenter2.property_changed({
        let observed = observed.clone();
        let presenter1 = presenter1.clone();
        move |e| {
            if e.property() == ContentPresenter::content_property().as_property() && observed.get().is_none() {
                observed.set(Some(presenter1.content().is_some()));
            }
        }
    });

    root.set_child(None::<Ref<Control>>);
    root.set_child(target.clone());
    subscription.dispose();

    observed.get().expect("the content of the second presenter was not set")
}

#[test]
fn last_presenter_is_cleared_first_when_it_holds_the_same_boxed_content() {
    let _app = start();
    let content = boxed_str("foo");
    let (target, _transition, root) = create_target(content.clone());
    target.set_page_transition(None);

    // The first presenter holds the very object that is the content.
    let held = target.presenter().unwrap().content().unwrap();
    assert!(Rc::ptr_eq(&held, content.as_ref().unwrap()));

    assert!(!first_presenter_has_content_when_second_is_set(&target, &root));
    assert_eq!(text("foo"), content_of(&get_content_presenters2(&target)));
    assert!(target.presenter().unwrap().content().is_none());
}

#[test]
fn last_presenter_is_not_cleared_first_when_it_holds_an_equal_value_boxed_separately() {
    fn number(value: i32) -> Option<BoxedValue> {
        Some(Rc::new(value))
    }

    let _app = start();
    let (target, _transition, root) = create_target(number(42));
    target.set_page_transition(None);

    // The first presenter holds an equal value in another box: not the same
    // reference as the content (two boxings of a value are two objects).
    let presenter1 = target.presenter().unwrap();
    let other = number(42);
    presenter1.set_content(number(7));
    presenter1.set_content(other.clone());
    assert!(Rc::ptr_eq(&presenter1.content().unwrap(), other.as_ref().unwrap()));
    assert!(!Rc::ptr_eq(&presenter1.content().unwrap(), target.content().as_ref().unwrap()));

    assert!(first_presenter_has_content_when_second_is_set(&target, &root));
    let shown = get_content_presenters2(&target).content().expect("the second presenter has the content");
    assert!(*shown == **other.as_ref().unwrap());
    // The old presenter is hidden afterwards, as always.
    assert!(target.presenter().unwrap().content().is_none());
}

/// Equal strings are one reference wherever they were boxed (see
/// `reference_equals`): the text is the same instance in the reference.
#[test]
fn last_presenter_is_cleared_first_when_it_holds_an_equal_string_boxed_separately() {
    let _app = start();
    let (target, _transition, root) = create_target(boxed_str("foo"));
    target.set_page_transition(None);

    let presenter1 = target.presenter().unwrap();
    let other = boxed_str("foo");
    presenter1.set_content(boxed_str("bar"));
    presenter1.set_content(other.clone());
    assert!(!Rc::ptr_eq(&presenter1.content().unwrap(), target.content().as_ref().unwrap()));

    assert!(!first_presenter_has_content_when_second_is_set(&target, &root));
    assert_eq!(text("foo"), content_of(&get_content_presenters2(&target)));
    assert!(target.presenter().unwrap().content().is_none());
}

#[test]
fn last_presenter_is_cleared_first_when_it_holds_the_same_control() {
    let _app = start();
    let control = Control::new();
    // The same control boxed twice is the same reference.
    let (target, _transition, root) = create_target(Some(Control::boxed(control.clone())));
    target.set_page_transition(None);
    let presenter1 = target.presenter().unwrap();
    presenter1.set_content(None);
    presenter1.set_content(Some(Control::boxed(control)));

    assert!(!first_presenter_has_content_when_second_is_set(&target, &root));
}
