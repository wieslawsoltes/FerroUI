//! The reference tests await the navigation methods; without a page
//! transition a navigation completes before the method returns, and with one
//! its continuation is a job of the dispatcher of the test. `wait` stands for
//! `await`: it runs the jobs of the dispatcher if the task has not completed
//! yet and takes the result.
//!
//! The upstream tests are grouped in nested classes; the groups are kept, in
//! order, as sections of this file and of the `navigation_page_tests_*`
//! files.

use super::{
    start_async, ContentPage, NavigatingFromEventArgs, NavigatingTask, NavigationEventArgs, NavigationPage,
    NavigationType, Page, PageImpl, PageNavigationExtensions,
};
use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControlImpl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::{Border, Button, ControlImpl, Panel};
use ferroui_base::animation::IPageTransition;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::{CancellationToken, Dispatcher, DispatcherTask};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, ObjectType, Ref, StaticType,
    StyledElement, StyledElementImpl, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

// --- helpers shared by the navigation page test files ---

/// `await task`.
pub(super) fn wait<T: 'static>(task: DispatcherTask<T>) -> T {
    if !task.is_completed() {
        Dispatcher::ui_thread().run_jobs(None);
    }
    assert!(task.is_completed(), "the navigation did not complete");
    task.result().expect("the navigation was not canceled")
}

/// `await Assert.ThrowsAsync(() => task)`.
pub(super) fn throws_async<T: 'static>(task: DispatcherTask<T>) -> bool {
    if !task.is_completed() {
        Dispatcher::ui_thread().run_jobs(None);
    }
    task.is_faulted()
}

/// `Assert.Throws(() => action())`.
pub(super) fn throws(action: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).is_err()
}

pub(super) fn page() -> Ref<ContentPage> {
    ContentPage::new()
}

/// `new ContentPage { Header = header }`.
pub(super) fn page_h(header: &str) -> Ref<ContentPage> {
    let page = ContentPage::new();
    page.set_header(boxed_str(header));
    page
}

/// The identity of an object reference, for `Assert.Same`.
pub(super) trait Identity {
    fn identity(&self) -> Option<*const ()>;
}

impl<T: ObjectType> Identity for Ref<T> {
    fn identity(&self) -> Option<*const ()> {
        Some(&**self as *const T as *const ())
    }
}

impl<T: ObjectType> Identity for Option<Ref<T>> {
    fn identity(&self) -> Option<*const ()> {
        self.as_ref().and_then(Identity::identity)
    }
}

/// `Assert.Same(expected, actual)` as a condition.
pub(super) fn same(expected: &impl Identity, actual: &impl Identity) -> bool {
    expected.identity().is_some() && expected.identity() == actual.identity()
}

/// Whether the navigation service of the page is the navigation page.
pub(super) fn navigation_is(page: &Page, nav: &NavigationPage) -> bool {
    page.navigation().is_some_and(|navigation| *navigation == *nav.as_navigation())
}

/// Whether the stack holds exactly these pages, in order.
pub(super) fn stack_is(stack: &[Ref<Page>], expected: &[&Ref<ContentPage>]) -> bool {
    stack.len() == expected.len() && stack.iter().zip(expected).all(|(actual, expected)| same(actual, *expected))
}

pub(super) fn logical_children_contain<T: ObjectType>(nav: &NavigationPage, page: &Ref<T>) -> bool {
    StyledElement::logical_children(nav).to_vec().iter().any(|child| same(child, page))
}

pub(super) type Slot<T> = Rc<RefCell<Option<T>>>;

pub(super) fn slot<T>() -> Slot<T> {
    Rc::new(RefCell::new(None))
}

/// A handler that stores the args it receives.
pub(super) fn store<T: Clone + 'static>(slot: &Slot<T>) -> impl Fn(&T) + 'static {
    let slot = slot.clone();
    move |e| *slot.borrow_mut() = Some(e.clone())
}

pub(super) fn taken<T: Clone>(slot: &Slot<T>) -> T {
    slot.borrow().clone().expect("the event was raised")
}

pub(super) fn flag() -> Rc<Cell<bool>> {
    Rc::new(Cell::new(false))
}

/// A handler that sets the flag.
pub(super) fn raise_flag<T>(flag: &Rc<Cell<bool>>) -> impl Fn(&T) + 'static {
    let flag = flag.clone();
    move |_| flag.set(true)
}

pub(super) fn counter() -> Rc<Cell<i32>> {
    Rc::new(Cell::new(0))
}

/// A handler that counts its calls.
pub(super) fn count<T>(counter: &Rc<Cell<i32>>) -> impl Fn(&T) + 'static {
    let counter = counter.clone();
    move |_| counter.set(counter.get() + 1)
}

pub(super) type Log = Rc<RefCell<Vec<&'static str>>>;

pub(super) fn log() -> Log {
    Rc::new(RefCell::new(Vec::new()))
}

/// A handler that adds the entry to the log.
pub(super) fn record<T>(log: &Log, entry: &'static str) -> impl Fn(&T) + 'static {
    let log = log.clone();
    move |_| log.borrow_mut().push(entry)
}

/// `Task.CompletedTask`.
pub(super) fn completed() -> NavigatingTask {
    Box::pin(std::future::ready(()))
}

/// A `Navigating` handler that cancels the navigation and sets the flag.
pub(super) fn cancel_navigation(
    invoked: &Rc<Cell<bool>>,
) -> impl Fn(&NavigatingFromEventArgs) -> NavigatingTask + 'static {
    let invoked = invoked.clone();
    move |args| {
        invoked.set(true);
        args.set_cancel(true);
        completed()
    }
}

/// Creates a templated navigation page in a root. The root is returned with
/// it: it keeps the navigation page attached while the test holds it.
pub(super) fn create_navigation_page(
    transition: Option<Rc<dyn IPageTransition>>,
) -> (Ref<NavigationPage>, Ref<TestRoot>) {
    let nav = NavigationPage::new();
    nav.set_page_transition(transition);
    nav.set_template(Some(create_navigation_page_template()));
    let root = TestRoot::with_child(nav.clone());
    root.execute_initial_layout_pass();
    (nav, root)
}

fn named_presenter(name: &str) -> Ref<ContentPresenter> {
    let presenter = ContentPresenter::new();
    presenter.set_name(Some(name.to_string()));
    presenter
}

pub(super) fn create_navigation_page_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let content_host = Panel::new();
        content_host.set_name(Some("PART_ContentHost".to_string()));
        content_host.children().add(named_presenter("PART_PageBackPresenter").register_in_name_scope(&**ns));
        content_host.children().add(named_presenter("PART_PagePresenter").register_in_name_scope(&**ns));
        let content_host = content_host.register_in_name_scope(&**ns);

        let back_button = Button::new();
        back_button.set_name(Some("PART_BackButton".to_string()));
        let navigation_bar = Border::new();
        navigation_bar.set_name(Some("PART_NavigationBar".to_string()));
        navigation_bar.set_child(back_button.register_in_name_scope(&**ns));

        let panel = Panel::new();
        panel.children().add(navigation_bar.register_in_name_scope(&**ns));
        panel.children().add(content_host);
        panel.children().add(named_presenter("PART_TopCommandBar").register_in_name_scope(&**ns));
        panel.children().add(named_presenter("PART_ModalBackPresenter").register_in_name_scope(&**ns));
        panel.children().add(named_presenter("PART_ModalPresenter").register_in_name_scope(&**ns));
        panel.upcast()
    })
}

/// The task completion source of the reference tests: a signal that a
/// transition waits for.
#[derive(Clone)]
pub(super) struct Gate(Rc<GateState>);

struct GateState {
    completed: Cell<bool>,
    wakers: RefCell<Vec<Waker>>,
}

impl Gate {
    pub(super) fn new() -> Self {
        Self(Rc::new(GateState { completed: Cell::new(false), wakers: RefCell::new(Vec::new()) }))
    }

    pub(super) fn set_result(&self) {
        self.0.completed.set(true);
        let wakers = std::mem::take(&mut *self.0.wakers.borrow_mut());
        for waker in wakers {
            waker.wake();
        }
    }

    pub(super) fn is_completed(&self) -> bool {
        self.0.completed.get()
    }
}

struct GateFuture(Gate);

impl Future for GateFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.is_completed() {
            return Poll::Ready(());
        }
        self.0 .0.wakers.borrow_mut().push(cx.waker().clone());
        Poll::Pending
    }
}

/// A transition that ends when its gate is opened.
pub(super) struct ControllableTransition {
    gate: Gate,
}

impl ControllableTransition {
    pub(super) fn new(gate: &Gate) -> Rc<dyn IPageTransition> {
        Rc::new(Self { gate: gate.clone() })
    }
}

impl IPageTransition for ControllableTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        _forward: bool,
        _cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let from = from.cloned();
        let to = to.cloned();
        let gate = self.gate.clone();
        start_async(async move {
            if let Some(to) = &to {
                to.set_is_visible(true);
            }
            GateFuture(gate).await;
            if let Some(from) = &from {
                from.set_is_visible(false);
            }
        })
    }
}

/// A content page that counts the presses of the system back button and
/// handles them on demand.
#[repr(C)]
pub(super) struct BackHandlingPage {
    base: ContentPage,
    back_button_press_count: Cell<i32>,
    handle_back: Cell<bool>,
}

ferro_class!(BackHandlingPage: ContentPage);
ferro_impl_classes!(
    BackHandlingPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl PageImpl for BackHandlingPage {
    fn on_system_back_button_pressed(this: &Self) -> bool {
        this.back_button_press_count.set(this.back_button_press_count.get() + 1);
        this.handle_back.get()
    }
}

impl BackHandlingPage {
    pub(super) fn new(handle_back: bool) -> Ref<Self> {
        instantiate(Self {
            base: ContentPage::construct(),
            back_button_press_count: Cell::new(0),
            handle_back: Cell::new(handle_back),
        })
    }

    pub(super) fn back_button_press_count(&self) -> i32 {
        self.back_button_press_count.get()
    }
}

/// A content page that cancels every navigation away from it.
#[repr(C)]
pub(super) struct CancellingPage {
    base: ContentPage,
}

ferro_class!(CancellingPage: ContentPage);
ferro_impl_classes!(
    CancellingPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl PageImpl for CancellingPage {
    fn on_navigating_from(_this: &Self, args: &NavigatingFromEventArgs) {
        args.set_cancel(true);
    }
}

impl CancellingPage {
    pub(super) fn new() -> Ref<Self> {
        instantiate(Self { base: ContentPage::construct() })
    }
}

fn is_content_page(page: &Option<Ref<Page>>) -> bool {
    page.as_ref().is_some_and(|page| std::ptr::eq(page.get_type(), <ContentPage as StaticType>::TYPE))
}

// --- PushTests ---

#[test]
fn push_single_page_stack_depth_becomes_one() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    assert_eq!(1, nav.stack_depth());
}

#[test]
fn push_multiple_times_stack_depth_matches_count() {
    let _scope = test_scope();
    for n in [1, 3, 10] {
        let nav = NavigationPage::new();
        for _ in 0..n {
            wait(nav.push_async(page_h("Page")));
        }
        assert_eq!(n, nav.stack_depth());
    }
}

#[test]
fn push_sets_current_page_to_top_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));
    assert!(same(&top, &nav.current_page()));
}

#[test]
fn push_sets_is_in_navigation_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let page = page();
    assert!(!page.is_in_navigation_page());
    wait(nav.push_async(&page));
    assert!(page.is_in_navigation_page());
}

#[test]
fn push_sets_navigation_property() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let page = page();
    wait(nav.push_async(&page));
    assert!(navigation_is(&page, &nav));
}

#[test]
fn push_duplicate_page_throws() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let page = page();
    wait(nav.push_async(&page));
    assert!(throws_async(nav.push_async(&page)));
}

#[test]
fn push_page_already_presented_modally_throws() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));

    let modal = page();
    wait(nav.push_modal_async(&modal));

    assert!(throws_async(nav.push_async(&modal)));
}

#[test]
fn push_fires_pushed_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let received = slot::<NavigationEventArgs>();
    nav.pushed(store(&received));

    let page = page();
    wait(nav.push_async(&page));

    let received = taken(&received);
    assert!(same(&page, &received.page()));
    assert_eq!(NavigationType::Push, received.navigation_type());
}

#[test]
fn push_invokes_navigated_to_on_pushed_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));

    let args = slot();
    let second = page();
    second.navigated_to(store(&args));

    wait(nav.push_async(&second));

    let args = taken(&args);
    assert!(same(&root, &args.previous_page()));
    assert_eq!(NavigationType::Push, args.navigation_type());
}

#[test]
fn push_invokes_navigated_from_on_previous_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let args = slot();
    root.navigated_from(store(&args));
    wait(nav.push_async(&root));

    let second = page();
    wait(nav.push_async(&second));

    let args = taken(&args);
    assert!(same(&second, &args.destination_page()));
    assert_eq!(NavigationType::Push, args.navigation_type());
}

#[test]
fn push_async_when_navigating_from_cancels_does_not_push() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));

    root.navigating(cancel_navigation(&flag()));

    wait(nav.push_async(page()));

    assert_eq!(1, nav.stack_depth());
    assert!(same(&root, &nav.current_page()));
}

#[test]
fn push_reentrant_from_navigated_to_is_ignored_not_thrown() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));

    let second = page();
    let weak = nav.downgrade();
    second.navigated_to(move |_| {
        let nav = weak.upgrade().expect("the navigation page is alive");
        wait(nav.push_async(page()));
    });

    wait(nav.push_async(&second));

    assert_eq!(2, nav.stack_depth());
    assert!(same(&second, &nav.current_page()));
}

fn same_value(a: &Option<BoxedValue>, b: &BoxedValue) -> bool {
    a.as_ref().is_some_and(|a| Rc::ptr_eq(a, b))
}

#[test]
fn push_passes_parameter_with_event_args() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let param: BoxedValue = Rc::new(());
    let args = slot();
    root.navigated_to(store(&args));
    wait(nav.push_async_with_parameter(&root, None, Some(param.clone())));
    assert!(same_value(&taken(&args).parameter(), &param));
}

#[test]
fn push_generic_pushes_page_with_correct_type() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.as_navigation().push_async_of::<ContentPage>(None, None));

    assert!(is_content_page(&nav.current_page()));
}

#[test]
fn push_modal_passes_parameter_with_event_args() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let param: BoxedValue = Rc::new(());
    let args = slot();
    root.navigated_to(store(&args));
    wait(nav.push_modal_async_with_parameter(&root, None, Some(param.clone())));
    assert!(same_value(&taken(&args).parameter(), &param));
}

#[test]
fn push_modal_generic_pushes_page_with_correct_type() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.as_navigation().push_modal_async_of::<ContentPage>(None, None));

    let modal_content = nav
        .modal_content()
        .as_ref()
        .and_then(crate::Control::from_boxed)
        .and_then(|control| control.cast::<Page>());
    assert!(is_content_page(&modal_content));
}

// --- ReentrantNavigationTests ---

#[test]
fn pop_reentrant_from_navigated_to_is_ignored() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let weak = nav.downgrade();
    root.navigated_to(move |_| {
        let nav = weak.upgrade().expect("the navigation page is alive");
        wait(nav.pop_async());
    });

    wait(nav.pop_async());

    assert_eq!(1, nav.stack_depth());
    assert!(same(&root, &nav.current_page()));
}

#[test]
fn push_modal_reentrant_from_navigated_to_is_ignored() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));

    let modal = page();
    let weak = nav.downgrade();
    modal.navigated_to(move |_| {
        let nav = weak.upgrade().expect("the navigation page is alive");
        wait(nav.push_modal_async(page()));
    });

    wait(nav.push_modal_async(&modal));

    assert!(stack_is(&nav.modal_stack(), &[&modal]));
}

#[test]
fn pop_modal_reentrant_from_navigated_to_is_ignored() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let modal = page();
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(&modal));

    let weak = nav.downgrade();
    root.navigated_to(move |_| {
        let nav = weak.upgrade().expect("the navigation page is alive");
        wait(nav.pop_modal_async());
    });

    wait(nav.pop_modal_async());

    assert_eq!(0, nav.modal_stack().len());
    assert!(same(&root, &nav.current_page()));
}

// --- PopTests ---

#[test]
fn pop_on_empty_stack_returns_null() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    assert!(wait(nav.pop_async()).is_none());
}

#[test]
fn pop_on_root_only_returns_null_and_keeps_root() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let result = wait(nav.pop_async());
    assert!(result.is_none());
    assert_eq!(1, nav.stack_depth());
    assert!(same(&root, &nav.current_page()));
}

#[test]
fn pop_returns_popped_page_and_decrements_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let result = wait(nav.pop_async());

    assert!(same(&top, &result));
    assert_eq!(1, nav.stack_depth());
    assert!(same(&root, &nav.current_page()));
}

#[test]
fn pop_clears_is_in_navigation_page_on_popped_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    wait(nav.pop_async());

    assert!(!top.is_in_navigation_page());
    assert!(top.navigation().is_none());
}

#[test]
fn pop_fires_popped_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let received = slot::<NavigationEventArgs>();
    nav.popped(store(&received));

    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));
    wait(nav.pop_async());

    let received = taken(&received);
    assert!(same(&top, &received.page()));
    assert_eq!(NavigationType::Pop, received.navigation_type());
}

#[test]
fn pop_invokes_navigated_to_on_revealed_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let args = slot();
    root.navigated_to(store(&args));
    wait(nav.push_async(&root));

    let top = page();
    wait(nav.push_async(&top));
    wait(nav.pop_async());

    let args = taken(&args);
    assert!(same(&top, &args.previous_page()));
    assert_eq!(NavigationType::Pop, args.navigation_type());
}

#[test]
fn pop_async_when_navigating_from_cancels_does_not_pop() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let top = page();
    wait(nav.push_async(&top));

    top.navigating(cancel_navigation(&flag()));

    wait(nav.pop_async());

    assert_eq!(2, nav.stack_depth());
    assert!(same(&top, &nav.current_page()));
}

#[test]
fn pop_async_invokes_navigated_to_on_revealed_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let top = page();
    wait(nav.push_async(&top));

    let navigated_to = flag();
    root.navigated_to(raise_flag(&navigated_to));

    wait(nav.pop_async());

    assert!(navigated_to.get());
}

// --- NavigationStackTests ---

#[test]
fn navigation_stack_root_at_index_zero_top_at_last_index() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let middle = page_h("Middle");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&middle));
    wait(nav.push_async(&top));

    let stack = nav.navigation_stack();
    assert_eq!(3, stack.len());
    assert!(same(&root, &stack[0]));
    assert!(same(&middle, &stack[1]));
    assert!(same(&top, &stack[2]));
}

#[test]
fn can_go_back_false_with_one_entry() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    assert!(!nav.can_go_back());
}

#[test]
fn can_go_back_true_with_two_entries() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    wait(nav.push_async(page()));
    assert!(nav.can_go_back());
}

#[test]
fn can_go_back_false_after_pop() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    wait(nav.push_async(page()));
    wait(nav.pop_async());
    assert!(!nav.can_go_back());
}

#[test]
fn stack_depth_always_equals_navigation_stack_count() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    assert_eq!(nav.navigation_stack().len() as i32, nav.stack_depth());

    wait(nav.push_async(page()));
    assert_eq!(nav.navigation_stack().len() as i32, nav.stack_depth());

    wait(nav.pop_async());
    assert_eq!(nav.navigation_stack().len() as i32, nav.stack_depth());
}

// --- BackButtonVisibilityTests ---

#[test]
fn back_button_visible_false_when_stack_depth_is_one() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    assert!(!nav.is_back_button_effectively_visible());
}

#[test]
fn back_button_visible_true_when_stack_depth_is_two() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    wait(nav.push_async(page()));
    assert!(nav.is_back_button_effectively_visible());
}

#[test]
fn back_button_visible_false_when_is_back_button_visible_is_false() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    nav.set_is_back_button_visible(false);
    wait(nav.push_async(page()));
    wait(nav.push_async(page()));
    assert!(!nav.is_back_button_effectively_visible());
}

#[test]
fn back_button_visible_false_when_per_page_is_back_button_visible_is_false() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let top = page();
    NavigationPage::set_has_back_button(&top, false);
    wait(nav.push_async(&top));
    assert!(!nav.is_back_button_effectively_visible());
}

#[test]
fn back_button_visible_true_after_restoring_global_visibility() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    nav.set_is_back_button_visible(false);
    wait(nav.push_async(page()));
    wait(nav.push_async(page()));
    nav.set_is_back_button_visible(true);
    assert!(nav.is_back_button_effectively_visible());
}

#[test]
fn back_button_visible_updates_when_current_page_has_back_button_changes() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));

    let top = page();
    wait(nav.push_async(&top));

    assert!(nav.is_back_button_effectively_visible());

    NavigationPage::set_has_back_button(&top, false);
    assert!(!nav.is_back_button_effectively_visible());

    NavigationPage::set_has_back_button(&top, true);
    assert!(nav.is_back_button_effectively_visible());
}
