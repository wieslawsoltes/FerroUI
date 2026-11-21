//! The reference tests host their controls in a stack panel; the test root
//! of this crate lays out its visual children directly.

use super::{
    BringIntoViewRequest, IBringIntoViewLayoutManager, ILayoutManager, LayoutManager, Layoutable, LayoutableImpl,
    LayoutableImplExt,
};
use crate::threading::Dispatcher;
use crate::tree_tests::{TestRoot, TestSource};
use crate::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Size, StyledElementImpl, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A control that records whether it has been measured and arranged.
#[repr(C)]
struct LayoutTestControl {
    base: Layoutable,
    measured: Cell<bool>,
    arranged: Cell<bool>,
}

ferro_class!(LayoutTestControl: Layoutable);
ferro_impl_classes!(LayoutTestControl: FerroObjectImpl, StyledElementImpl, VisualImpl);

impl LayoutableImpl for LayoutTestControl {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.measured.set(true);
        Self::parent_measure_override(this, available_size)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.arranged.set(true);
        Self::parent_arrange_override(this, final_size)
    }
}

impl LayoutTestControl {
    fn new() -> Ref<Self> {
        instantiate(Self { base: Layoutable::construct(), measured: Cell::new(false), arranged: Cell::new(false) })
    }
}

struct TestRequest {
    target: Ref<Layoutable>,
    execute_attempts: Cell<i32>,
    executions: Cell<i32>,
    can_execute: RefCell<Option<Rc<dyn Fn() -> bool>>>,
    on_execute: RefCell<Option<Rc<dyn Fn()>>>,
}

impl TestRequest {
    fn new(target: &Ref<LayoutTestControl>) -> Rc<Self> {
        Rc::new(Self {
            target: target.clone().upcast(),
            execute_attempts: Cell::new(0),
            executions: Cell::new(0),
            can_execute: RefCell::new(None),
            on_execute: RefCell::new(None),
        })
    }

    fn with_can_execute(self: Rc<Self>, can_execute: impl Fn() -> bool + 'static) -> Rc<Self> {
        *self.can_execute.borrow_mut() = Some(Rc::new(can_execute));
        self
    }

    fn with_on_execute(self: Rc<Self>, on_execute: impl Fn() + 'static) -> Rc<Self> {
        *self.on_execute.borrow_mut() = Some(Rc::new(on_execute));
        self
    }
}

impl BringIntoViewRequest for TestRequest {
    fn target(&self) -> Ref<Layoutable> {
        self.target.clone()
    }

    fn try_execute(&self) -> bool {
        self.execute_attempts.set(self.execute_attempts.get() + 1);

        let can_execute = self.can_execute.borrow().clone();
        let can_execute = can_execute.map_or(true, |can_execute| can_execute());
        if !can_execute {
            return false;
        }

        self.executions.set(self.executions.get() + 1);
        let on_execute = self.on_execute.borrow().clone();
        if let Some(on_execute) = on_execute {
            on_execute();
        }
        true
    }
}

fn add_child(root: &Ref<TestRoot>, child: &Ref<LayoutTestControl>) {
    root.logical_children().add(child.clone().upcast());
    root.visual_children().add(child.clone().upcast());
}

fn remove_child(root: &Ref<TestRoot>, child: &Ref<LayoutTestControl>) {
    root.visual_children().remove(&child.clone().upcast());
    root.logical_children().remove(&child.clone().upcast());
}

fn create_root(children: &[&Ref<LayoutTestControl>]) -> (Ref<TestRoot>, Rc<TestSource>, Rc<LayoutManager>) {
    let root = TestRoot::new();
    for child in children {
        add_child(&root, child);
    }
    let source = TestSource::new(root.clone(), 1.0);
    root.set_presentation_source_for_root_visual(Some(source.clone()));
    let layout_manager = source.manager();
    (root, source, layout_manager)
}

fn get_layout_manager(layout_manager: &Rc<LayoutManager>) -> &dyn IBringIntoViewLayoutManager {
    let layout_manager: &dyn ILayoutManager = &**layout_manager;
    layout_manager.as_bring_into_view_layout_manager().expect("a bring-into-view layout manager")
}

#[test]
fn request_is_executed_at_end_of_layout_pass() {
    let _scope = Dispatcher::unit_test_scope();
    let control = LayoutTestControl::new();
    let (_root, _source, layout_manager) = create_root(&[&control]);
    layout_manager.execute_initial_layout_pass();

    let request = TestRequest::new(&control);
    get_layout_manager(&layout_manager).enqueue_bring_into_view(request.clone());

    assert_eq!(request.execute_attempts.get(), 0);

    layout_manager.execute_layout_pass();

    assert_eq!(request.execute_attempts.get(), 1);
    assert_eq!(request.executions.get(), 1);

    layout_manager.execute_layout_pass();

    // Should not have been executed twice.
    assert_eq!(request.execute_attempts.get(), 1);
    assert_eq!(request.executions.get(), 1);
}

#[test]
fn request_is_executed_before_layout_updated_is_raised() {
    let _scope = Dispatcher::unit_test_scope();
    let control = LayoutTestControl::new();
    let (_root, _source, layout_manager) = create_root(&[&control]);
    layout_manager.execute_initial_layout_pass();

    let layout_updated_raised = Rc::new(Cell::new(false));
    let executed_before_layout_updated = Rc::new(Cell::new(false));
    let raised = layout_updated_raised.clone();
    layout_manager.add_layout_updated(Rc::new(move || raised.set(true)));

    let raised = layout_updated_raised.clone();
    let executed = executed_before_layout_updated.clone();
    let request = TestRequest::new(&control).with_on_execute(move || executed.set(!raised.get()));

    get_layout_manager(&layout_manager).enqueue_bring_into_view(request.clone());
    layout_manager.execute_layout_pass();

    assert_eq!(request.executions.get(), 1);
    assert!(executed_before_layout_updated.get());
}

#[test]
fn request_is_retained_until_it_can_execute() {
    let _scope = Dispatcher::unit_test_scope();
    let control = LayoutTestControl::new();
    let (_root, _source, layout_manager) = create_root(&[&control]);
    layout_manager.execute_initial_layout_pass();

    let can_execute = Rc::new(Cell::new(false));
    let can = can_execute.clone();
    let request = TestRequest::new(&control).with_can_execute(move || can.get());
    get_layout_manager(&layout_manager).enqueue_bring_into_view(request.clone());

    layout_manager.execute_layout_pass();

    assert_eq!(request.execute_attempts.get(), 1);
    assert_eq!(request.executions.get(), 0);

    layout_manager.execute_layout_pass();

    assert_eq!(request.execute_attempts.get(), 2);
    assert_eq!(request.executions.get(), 0);

    can_execute.set(true);
    layout_manager.execute_layout_pass();

    assert_eq!(request.execute_attempts.get(), 3);
    assert_eq!(request.executions.get(), 1);
}

#[test]
fn requests_are_coalesced_by_target() {
    let _scope = Dispatcher::unit_test_scope();
    let control = LayoutTestControl::new();
    let (_root, _source, layout_manager) = create_root(&[&control]);
    layout_manager.execute_initial_layout_pass();

    let first = TestRequest::new(&control);
    let second = TestRequest::new(&control);

    get_layout_manager(&layout_manager).enqueue_bring_into_view(first.clone());
    get_layout_manager(&layout_manager).enqueue_bring_into_view(second.clone());

    layout_manager.execute_layout_pass();

    assert_eq!(first.execute_attempts.get(), 0);
    assert_eq!(second.executions.get(), 1);
}

#[test]
fn request_is_dropped_when_target_is_detached() {
    let _scope = Dispatcher::unit_test_scope();
    let control = LayoutTestControl::new();
    let (root, _source, layout_manager) = create_root(&[&control]);
    layout_manager.execute_initial_layout_pass();

    let request = TestRequest::new(&control);
    get_layout_manager(&layout_manager).enqueue_bring_into_view(request.clone());

    remove_child(&root, &control);
    layout_manager.execute_layout_pass();

    assert_eq!(request.execute_attempts.get(), 0);

    add_child(&root, &control);
    layout_manager.execute_layout_pass();

    assert_eq!(request.execute_attempts.get(), 0);
}

#[test]
fn layout_invalidated_by_request_converges_within_same_pass() {
    let _scope = Dispatcher::unit_test_scope();
    let control = LayoutTestControl::new();
    let (_root, _source, layout_manager) = create_root(&[&control]);
    layout_manager.execute_initial_layout_pass();

    control.measured.set(false);
    control.arranged.set(false);

    let target = control.clone();
    let request = TestRequest::new(&control).with_on_execute(move || target.invalidate_measure());
    get_layout_manager(&layout_manager).enqueue_bring_into_view(request.clone());

    layout_manager.execute_layout_pass();

    // The layout invalidated by the request (e.g. a scroll offset change) has
    // been re-run before the layout pass returned, so the frame is rendered
    // fully scrolled.
    assert_eq!(request.executions.get(), 1);
    assert!(control.measured.get());
    assert!(control.arranged.get());
}

#[test]
fn layout_updated_is_not_raised_until_all_requests_have_been_processed() {
    let _scope = Dispatcher::unit_test_scope();
    let first = LayoutTestControl::new();
    let second = LayoutTestControl::new();
    let (_root, _source, layout_manager) = create_root(&[&first, &second]);
    layout_manager.execute_initial_layout_pass();

    let layout_updated_raised = Rc::new(Cell::new(false));
    let second_executed_before_layout_updated = Rc::new(Cell::new(false));
    let raised = layout_updated_raised.clone();
    layout_manager.add_layout_updated(Rc::new(move || raised.set(true)));

    // The second request can only execute once the layout invalidated by the
    // first one has been re-run, so it is executed by a following pass of the
    // processing loop.
    first.measured.set(false);

    let measured = first.clone();
    let raised = layout_updated_raised.clone();
    let executed = second_executed_before_layout_updated.clone();
    let second_request = TestRequest::new(&second)
        .with_can_execute(move || measured.measured.get())
        .with_on_execute(move || executed.set(!raised.get()));

    let target = first.clone();
    let first_request = TestRequest::new(&first).with_on_execute(move || target.invalidate_measure());

    get_layout_manager(&layout_manager).enqueue_bring_into_view(first_request);
    get_layout_manager(&layout_manager).enqueue_bring_into_view(second_request.clone());

    layout_manager.execute_layout_pass();

    assert_eq!(second_request.executions.get(), 1);
    assert!(second_executed_before_layout_updated.get());
}

#[test]
fn layout_updated_is_raised_once_when_request_invalidates_layout() {
    let _scope = Dispatcher::unit_test_scope();
    let control = LayoutTestControl::new();
    let (_root, _source, layout_manager) = create_root(&[&control]);
    layout_manager.execute_initial_layout_pass();

    let layout_updated = Rc::new(Cell::new(0));
    let updated = layout_updated.clone();
    layout_manager.add_layout_updated(Rc::new(move || updated.set(updated.get() + 1)));

    let target = control.clone();
    let request = TestRequest::new(&control).with_on_execute(move || target.invalidate_measure());
    get_layout_manager(&layout_manager).enqueue_bring_into_view(request.clone());

    layout_manager.execute_layout_pass();

    assert_eq!(request.executions.get(), 1);
    assert_eq!(layout_updated.get(), 1);
}

#[test]
fn request_enqueued_while_processing_is_attempted_by_the_same_pass() {
    let _scope = Dispatcher::unit_test_scope();
    let first = LayoutTestControl::new();
    let second = LayoutTestControl::new();
    let (_root, _source, layout_manager) = create_root(&[&first, &second]);
    layout_manager.execute_initial_layout_pass();

    let can_execute_second = Rc::new(Cell::new(false));
    let can = can_execute_second.clone();
    let second_request = TestRequest::new(&second).with_can_execute(move || can.get());

    // Executing a request can enqueue another one (as bringing a control into
    // view does): the new request must be attempted by the same pass, and
    // retained if it can't execute yet.
    let manager = layout_manager.clone();
    let enqueued = second_request.clone();
    let target = first.clone();
    let first_request = TestRequest::new(&first).with_on_execute(move || {
        get_layout_manager(&manager).enqueue_bring_into_view(enqueued.clone());
        target.invalidate_measure();
    });

    get_layout_manager(&layout_manager).enqueue_bring_into_view(first_request.clone());
    layout_manager.execute_layout_pass();

    assert_eq!(first_request.executions.get(), 1);

    // Attempted first during the same pass as the first request, then retried
    // once after a new layout pass.
    assert_eq!(second_request.execute_attempts.get(), 2);
    assert_eq!(second_request.executions.get(), 0);

    // The second request couldn't execute despite having been through an
    // extra layout pass. We can't retry forever (nothing has changed). The
    // request will be retried again on the next "natural" pass.
    can_execute_second.set(true);
    layout_manager.execute_layout_pass();

    assert_eq!(second_request.execute_attempts.get(), 3);
    assert_eq!(second_request.executions.get(), 1);

    // Break the cycle between the layout manager and the first request.
    layout_manager.dispose();
}
