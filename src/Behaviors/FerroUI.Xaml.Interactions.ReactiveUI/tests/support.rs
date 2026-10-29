//! Not from the reference: the support of the tests of the crate.
//!
//! The tests run in the unit test application of the framework, with the services of a styled
//! window (whose test theme gives windows a template that presents their content). The routers
//! and the commands of ReactiveUI notify on the immediate scheduler, so a navigation has happened
//! when the action returns.

use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObject, FerroObjectImpl, ObjectType, Ref, StyledElementImpl};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, Window};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactiveui::{reactive_object, IRoutableViewModel, IScreen, ReactiveObject, RoutableViewModel, RoutingState, RxSchedulers};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Starts the application of a test.
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    RxSchedulers::set_main_thread_scheduler(ImmediateScheduler::instance());
    UnitTestApplication::start(TestServices::styled_window())
}

/// A router whose navigation notifies on the immediate scheduler.
pub(crate) fn router() -> Rc<RoutingState> {
    RoutingState::with_scheduler(Some(ImmediateScheduler::instance()))
}

/// A routable view model that names itself with a segment.
pub(crate) struct TestViewModel {
    reactive: ReactiveObject,
    segment: String,
}

reactive_object!(TestViewModel);

impl TestViewModel {
    pub(crate) fn new(segment: &str) -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new(), segment: segment.to_owned() })
    }
}

impl IRoutableViewModel for TestViewModel {
    fn url_path_segment(&self) -> Option<String> {
        Some(self.segment.clone())
    }

    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        None
    }
}

/// The same view model (reference equality, as for the objects of .NET).
impl PartialEq for TestViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A routable view model of another type.
pub(crate) struct OtherViewModel {
    reactive: ReactiveObject,
}

reactive_object!(OtherViewModel);

impl OtherViewModel {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new() })
    }
}

impl IRoutableViewModel for OtherViewModel {
    fn url_path_segment(&self) -> Option<String> {
        Some("other".to_owned())
    }

    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        None
    }
}

/// The view model as a shared routable view model.
pub(crate) fn routable<VM: IRoutableViewModel + 'static>(view_model: &Rc<VM>) -> RoutableViewModel {
    view_model.clone()
}

/// The routable view model as an untyped value (the parameter of an action).
pub(crate) fn boxed_routable(view_model: RoutableViewModel) -> BoxedValue {
    Rc::new(view_model)
}

/// A router whose stack holds view models with these segments, oldest first.
pub(crate) fn router_with(segments: &[&str]) -> Rc<RoutingState> {
    let router = router();
    for segment in segments {
        router.navigation_stack().add(routable(&TestViewModel::new(segment)));
    }
    router
}

/// The segments of the view models of the stack of the router, oldest first.
pub(crate) fn segments(router: &RoutingState) -> Vec<String> {
    router.navigation_stack().to_vec().iter().map(|view_model| view_model.url_path_segment().unwrap_or_default()).collect()
}

/// The `bool` an action returned.
pub(crate) fn returned(result: Option<BoxedValue>) -> Option<bool> {
    result.and_then(|result| result.downcast_ref::<bool>().copied())
}

/// `new Window { Content = content }`.
pub(crate) fn window_with(content: Option<Ref<Control>>) -> Ref<Window> {
    let window = Window::new();
    if let Some(content) = content {
        window.set_content(Some(Control::boxed(content)));
    }
    window
}

/// The object as the root class (the `FerroObject` items of the collections).
pub(crate) fn object<T: ObjectType>(value: &Ref<T>) -> Ref<FerroObject> {
    value.clone().upcast()
}

/// The object as an untyped value (an `object` argument of the reference).
pub(crate) fn boxed<T: ObjectType>(value: &Ref<T>) -> BoxedValue {
    Rc::new(object(value))
}

/// An action that records its executions.
#[repr(C)]
pub(crate) struct RecordingAction {
    base: StyledElementAction,
    sender: RefCell<Option<BoxedValue>>,
    parameter: RefCell<Option<BoxedValue>>,
    execute_count: Cell<i32>,
}

ferro_class!(RecordingAction: StyledElementAction);
ferroui_base::ferro_impl_classes!(RecordingAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for RecordingAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        this.execute_count.set(this.execute_count.get() + 1);
        *this.sender.borrow_mut() = sender.cloned();
        *this.parameter.borrow_mut() = parameter.cloned();
        None
    }
}

impl RecordingAction {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: StyledElementAction::construct(),
            sender: RefCell::new(None),
            parameter: RefCell::new(None),
            execute_count: Cell::new(0),
        })
    }

    pub(crate) fn sender(&self) -> Option<BoxedValue> {
        self.sender.borrow().clone()
    }

    pub(crate) fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.borrow().clone()
    }

    pub(crate) fn execute_count(&self) -> i32 {
        self.execute_count.get()
    }
}
