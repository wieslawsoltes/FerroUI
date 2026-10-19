//! `BuilderMixinsTests` (`ReactiveUI.Builder.Tests/Mixins/BuilderMixinsTests.cs`).
//!
//! The class runs in its `ResetOnlyExecutor`: each test configures and builds its builder.
//! `IsSameReferenceAs(builder)` compares addresses, and `IsAssignableTo<T>()` is a downcast.

use super::super::executors::reset_only;
use ferroui_reactive::concurrency::{CurrentThreadScheduler, IScheduler, ImmediateScheduler};
use ferroui_reactive::linq::Observable;
use ferroui_reactiveui::*;
use ferroui_splat::{AppLocator, IAppBuilder, IModule, IMutableDependencyResolver, MutableDependencyResolverExt};
use ferroui_system::IObservable;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `BuilderMixinsTestViewModel`.
#[derive(Default)]
pub struct BuilderMixinsTestViewModel {
    reactive: ReactiveObject,
}
reactive_object!(BuilderMixinsTestViewModel);

/// `BuilderMixinsTestView`.
#[derive(Default)]
pub struct BuilderMixinsTestView {
    view_model: RefCell<Option<Rc<BuilderMixinsTestViewModel>>>,
}
view_for!(BuilderMixinsTestView, BuilderMixinsTestViewModel, view_model);

/// `TestViewModule`.
#[derive(Default)]
struct TestViewModule;

impl IViewModule for TestViewModule {
    fn register_views(&self, locator: &DefaultViewLocator) {
        locator.map::<BuilderMixinsTestViewModel, BuilderMixinsTestView>(None);
    }
}

/// `PlatformRegistrationMarker`.
struct PlatformRegistrationMarker;

/// `SplatModuleMarker`.
struct SplatModuleMarker;

/// `TestRegistrationModule`.
#[derive(Default)]
struct TestRegistrationModule;

impl IWantsToRegisterStuff for TestRegistrationModule {
    fn register(&self, registrar: &dyn IRegistrar) {
        registrar.register_constant(|| Rc::new(PlatformRegistrationMarker));
    }
}

/// `TestSplatModule`.
#[derive(Default)]
struct TestSplatModule {
    /// `Registered`.
    registered: Cell<bool>,
}

impl IModule for TestSplatModule {
    fn configure(&self, resolver: &dyn IMutableDependencyResolver) {
        resolver.register_constant(Rc::new(SplatModuleMarker));
        self.registered.set(true);
    }
}

/// `TestSuspensionDriver`.
struct TestSuspensionDriver;

impl ISuspensionDriver for TestSuspensionDriver {
    fn load_state(&self) -> Rc<dyn IObservable<Option<Rc<dyn Any>>>> {
        Observable::return_(None)
    }

    fn save_state(&self, _state: Option<Rc<dyn Any>>) -> Rc<dyn IObservable<()>> {
        Observable::return_(())
    }

    fn invalidate_state(&self) -> Rc<dyn IObservable<()>> {
        Observable::return_(())
    }
}

fn immediate() -> Rc<dyn IScheduler> {
    ImmediateScheduler::instance()
}

fn same_scheduler(left: &Option<Rc<dyn IScheduler>>, right: &Rc<dyn IScheduler>) -> bool {
    left.as_ref().is_some_and(|left| std::ptr::addr_eq(Rc::as_ptr(left), Rc::as_ptr(right)))
}

fn same(left: &Rc<dyn IScheduler>, right: &Rc<dyn IScheduler>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(left), Rc::as_ptr(right))
}

// Not ported: UsingSplatBuilder_throws_when_builder_null - argument validation

#[test]
fn with_task_pool_scheduler_sets_scheduler_and_rx_schedulers() {
    reset_only(|| {
        let original = RxSchedulers::task_pool_scheduler();
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let scheduler = immediate();
        let _ = builder.with_task_pool_scheduler(scheduler.clone());
        let _ = builder.with_core_services().build();

        assert!(same_scheduler(&builder.task_pool_scheduler(), &scheduler));
        assert!(same(&RxSchedulers::task_pool_scheduler(), &scheduler));
        RxSchedulers::set_task_pool_scheduler(original);
    });
}

#[test]
fn with_main_thread_scheduler_sets_scheduler_and_rx_schedulers() {
    reset_only(|| {
        let original = RxSchedulers::main_thread_scheduler();
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let scheduler = immediate();
        let _ = builder.with_main_thread_scheduler(scheduler.clone());
        let _ = builder.with_core_services().build();

        assert!(same_scheduler(&builder.main_thread_scheduler(), &scheduler));
        assert!(same(&RxSchedulers::main_thread_scheduler(), &scheduler));
        RxSchedulers::set_main_thread_scheduler(original);
    });
}

#[test]
fn with_registration_on_build_registers_service_when_building() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.with_registration_on_build(|resolver: &dyn IMutableDependencyResolver| {
            resolver.register_constant(Rc::new("mixins".to_owned()));
        });
        let _ = builder.with_core_services().build();

        assert_eq!(AppLocator::get_service::<String>().as_deref().map(String::as_str), Some("mixins"));
    });
}

#[test]
fn with_registration_registers_service_immediately() {
    reset_only(|| {
        const EXPECTED_REGISTERED_VALUE: i32 = 42;
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.with_registration(|resolver| resolver.register_constant(Rc::new(EXPECTED_REGISTERED_VALUE)));

        assert_eq!(AppLocator::get_service::<i32>().map(|value| *value), Some(EXPECTED_REGISTERED_VALUE));
    });
}

// Not ported: WithViewsFromAssembly_registers_views_in_resolver - reflection

#[test]
fn with_platform_module_registers_module_types() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.with_platform_module::<TestRegistrationModule>();
        let _ = builder.with_core_services().build();

        assert!(AppLocator::get_service::<PlatformRegistrationMarker>().is_some());
    });
}

#[test]
fn using_splat_module_invokes_module_registration() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let module = Rc::new(TestSplatModule::default());
        let _ = builder.using_splat_module(module.clone());
        let _ = builder.with_core_services().build();

        assert!(AppLocator::get_service::<SplatModuleMarker>().is_some());
        assert!(module.registered.get());
    });
}

#[test]
fn using_splat_builder_executes_callback() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let invoked = Cell::new(false);
        let _ = builder.using_splat_builder(Some(&|_: &dyn IAppBuilder| invoked.set(true)));

        assert!(invoked.get());
    });
}

#[test]
fn using_splat_builder_handles_null_callback() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();

        let result = builder.using_splat_builder(None);

        assert!(std::ptr::eq(result, &builder));
    });
}

#[test]
fn for_custom_platform_sets_scheduler_and_platform_registrations_on_build() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let scheduler = immediate();
        let _ = builder.for_custom_platform(scheduler.clone(), |resolver: &dyn IMutableDependencyResolver| {
            resolver.register_constant(Rc::new(PlatformRegistrationMarker));
        });
        let registered_before_build = AppLocator::get_service::<PlatformRegistrationMarker>();
        let _ = builder.with_core_services().build();

        assert!(same_scheduler(&builder.main_thread_scheduler(), &scheduler));
        assert!(registered_before_build.is_none());
        assert!(AppLocator::get_service::<PlatformRegistrationMarker>().is_some());
    });
}

#[test]
fn for_platforms_invokes_all_actions() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let executed = RefCell::new(Vec::new());
        let first = |_: &dyn IReactiveUIBuilder| executed.borrow_mut().push("first");
        let second = |_: &dyn IReactiveUIBuilder| executed.borrow_mut().push("second");
        let platforms: [&dyn Fn(&dyn IReactiveUIBuilder); 2] = [&first, &second];
        let _ = builder.for_platforms(&platforms);

        assert_eq!(*executed.borrow(), vec!["first", "second"]);
    });
}

#[test]
fn configure_message_bus_registers_configured_instance() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let configured = Rc::new(Cell::new(false));
        let flag = configured.clone();
        let _ = builder.with_message_bus_with(move |_| flag.set(true));
        let _ = builder.with_core_services().build();

        // `IsAssignableTo<MessageBus>()`: the bus is a trait object, whose type cannot be read.
        assert!(AppLocator::get_service::<dyn IMessageBus>().is_some());
        assert!(configured.get());
    });
}

#[test]
fn configure_view_locator_registers_configured_locator() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let configured = Rc::new(Cell::new(false));
        let flag = configured.clone();
        let _ = builder.configure_view_locator(move |_| flag.set(true));
        let _ = builder.with_core_services().build();

        let locator = AppLocator::get_service::<dyn IViewLocator>().unwrap();
        assert!((locator as Rc<dyn Any>).downcast::<DefaultViewLocator>().is_ok());
        assert!(configured.get());
    });
}

#[test]
fn configure_suspension_driver_invokes_action_when_driver_registered() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let driver: Rc<dyn ISuspensionDriver> = Rc::new(TestSuspensionDriver);
        AppLocator::current_mutable().register_constant::<dyn ISuspensionDriver>(driver.clone());

        let observed: Rc<RefCell<Option<Rc<dyn ISuspensionDriver>>>> = Rc::new(RefCell::new(None));
        let sink = observed.clone();
        let _ = builder.configure_suspension_driver(move |d| *sink.borrow_mut() = Some(d.clone()));
        let _ = builder.with_core_services().build();

        let observed = observed.borrow().clone().expect("the action ran");
        assert!(Rc::ptr_eq(&observed, &driver));
    });
}

#[test]
fn register_view_model_registers_transient_view_model() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.register_view_model::<BuilderMixinsTestViewModel>();

        let first = AppLocator::get_service::<BuilderMixinsTestViewModel>().unwrap();
        let second = AppLocator::get_service::<BuilderMixinsTestViewModel>().unwrap();

        assert!(!Rc::ptr_eq(&first, &second));
    });
}

#[test]
fn register_singleton_view_model_registers_singleton_instance() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.register_singleton_view_model::<BuilderMixinsTestViewModel>();

        let first = AppLocator::get_service::<BuilderMixinsTestViewModel>().unwrap();
        let second = AppLocator::get_service::<BuilderMixinsTestViewModel>().unwrap();

        assert!(Rc::ptr_eq(&first, &second));
    });
}

#[test]
fn register_view_registers_transient_view() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.register_view::<BuilderMixinsTestView, BuilderMixinsTestViewModel>();

        let first = AppLocator::get_service::<dyn IViewForOf<BuilderMixinsTestViewModel>>().unwrap();
        let second = AppLocator::get_service::<dyn IViewForOf<BuilderMixinsTestViewModel>>().unwrap();

        assert!(!Rc::ptr_eq(&first, &second));
    });
}

#[test]
fn register_singleton_view_registers_singleton_view() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.register_singleton_view::<BuilderMixinsTestView, BuilderMixinsTestViewModel>();

        let first = AppLocator::get_service::<dyn IViewForOf<BuilderMixinsTestViewModel>>().unwrap();
        let second = AppLocator::get_service::<dyn IViewForOf<BuilderMixinsTestViewModel>>().unwrap();

        assert!(Rc::ptr_eq(&first, &second));
    });
}

// The additional tests of the `WithInstance` overloads.

#[test]
fn with_instance_single_parameter_invokes_action_with_resolved_instance() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        AppLocator::current_mutable().register_constant(Rc::new("test-value".to_owned()));
        let captured = RefCell::new(None);

        let _ = builder.with_instance(|value: Option<Rc<String>>| *captured.borrow_mut() = value);
        let _ = builder.with_core_services().build();

        assert_eq!(captured.borrow().as_deref().map(String::as_str), Some("test-value"));
    });
}

#[test]
fn with_instance_two_parameters_invokes_action_with_both_instances() {
    reset_only(|| {
        const EXPECTED_INT_INSTANCE: i32 = 42;
        let builder = RxAppBuilder::create_reactive_ui_builder();
        AppLocator::current_mutable().register_constant(Rc::new("string-value".to_owned()));
        AppLocator::current_mutable().register_constant(Rc::new(EXPECTED_INT_INSTANCE));
        let captured_string = RefCell::new(None);
        let captured_int = RefCell::new(None);

        let _ = builder.with_instance(|(text, number): (Option<Rc<String>>, Option<Rc<i32>>)| {
            *captured_string.borrow_mut() = text;
            *captured_int.borrow_mut() = number;
        });
        let _ = builder.with_core_services().build();

        assert_eq!(captured_string.borrow().as_deref().map(String::as_str), Some("string-value"));
        assert_eq!(captured_int.borrow().as_deref().copied(), Some(EXPECTED_INT_INSTANCE));
    });
}

// Not ported: WithInstance_WithNullBuilder_ThrowsArgumentNullException - argument validation
// Not ported: RegisterViews_WithNullBuilder_ThrowsArgumentNullException - argument validation
// Not ported: RegisterViews_WithNullConfigure_ThrowsArgumentNullException - argument validation

#[test]
fn register_views_with_view_locator_returns_builder() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        // The core services register the default view locator.
        let _ = builder.with_core_services().build();

        let result = builder.register_views(|views| {
            let _ = views.map::<BuilderMixinsTestViewModel, BuilderMixinsTestView>(None);
        });

        assert!(std::ptr::eq(result, &builder));
    });
}

// Not ported: WithViewModule_WithNullBuilder_ThrowsArgumentNullException - argument validation

#[test]
fn with_view_module_returns_builder() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        // The core services register the default view locator.
        let _ = builder.with_core_services().build();

        let result = builder.with_view_module::<TestViewModule>();

        assert!(std::ptr::eq(result, &builder));
    });
}

// Not ported: BuildApp_WithNullBuilder_ThrowsArgumentNullException - argument validation
// Not ported: BuildApp_WithNonReactiveUIBuilder_ThrowsInvalidOperationException - reflection (BuildApp of an IAppBuilder casts it to IReactiveUIBuilder)
// Not ported: BuildApp_WithReactiveUIBuilder_BuildsSuccessfully - reflection (BuildApp of an IAppBuilder casts it to IReactiveUIBuilder)

#[test]
fn with_message_bus_registers_custom_message_bus() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let custom_message_bus: Rc<dyn IMessageBus> = MessageBus::new();
        let _ = builder.with_message_bus_instance(custom_message_bus.clone());
        let _ = builder.with_core_services().build();

        let registered = AppLocator::get_service::<dyn IMessageBus>().unwrap();
        assert!(Rc::ptr_eq(&registered, &custom_message_bus));
    });
}

#[test]
fn with_task_pool_scheduler_with_set_rx_app_false_does_not_set_rx_schedulers() {
    reset_only(|| {
        let original = RxSchedulers::task_pool_scheduler();
        // A known baseline scheduler.
        let baseline_scheduler = CurrentThreadScheduler::instance();
        RxSchedulers::set_task_pool_scheduler(baseline_scheduler.clone());

        let builder = RxAppBuilder::create_reactive_ui_builder();
        let scheduler = immediate();
        let _ = builder.with_task_pool_scheduler_with(scheduler.clone(), false);
        let _ = builder.with_core_services().build();

        assert!(same_scheduler(&builder.task_pool_scheduler(), &scheduler));
        // `RxSchedulers` keeps the baseline.
        assert!(same(&RxSchedulers::task_pool_scheduler(), &baseline_scheduler));
        RxSchedulers::set_task_pool_scheduler(original);
    });
}

#[test]
fn with_main_thread_scheduler_with_set_rx_app_false_does_not_set_rx_schedulers() {
    reset_only(|| {
        let original = RxSchedulers::main_thread_scheduler();
        // A known baseline scheduler.
        let baseline_scheduler = CurrentThreadScheduler::instance();
        RxSchedulers::set_main_thread_scheduler(baseline_scheduler.clone());

        let builder = RxAppBuilder::create_reactive_ui_builder();
        let scheduler = immediate();
        let _ = builder.with_main_thread_scheduler_with(scheduler.clone(), false);
        let _ = builder.with_core_services().build();

        assert!(same_scheduler(&builder.main_thread_scheduler(), &scheduler));
        // `RxSchedulers` keeps the baseline.
        assert!(same(&RxSchedulers::main_thread_scheduler(), &baseline_scheduler));
        RxSchedulers::set_main_thread_scheduler(original);
    });
}
