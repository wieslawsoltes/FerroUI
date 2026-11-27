//! `AppBuilderExtensionsTests` (`AppBuilderExtensionsTests.cs`): the core behaviour of the
//! extensions of the application builder.

use super::view_support::{reset_reactive_ui, start};
use crate::{
    AutoDataTemplateBindingHook, FerroActivationForViewFetcher, FerroObjectHandles, FerroViewThreadInvoker,
    ReactiveUIBuilderFerroExtensions, ReactiveUserControlBase, ReactiveViewFor, StartupConfiguration,
};
use ferroui_controls::ListBox;
use ferroui_reactiveui::{
    BindingDirection, ICreatesCommandBinding, ICreatesObservableForProperty, IActivationForViewFetcher, IPropertyBindingHook,
    IViewThreadInvoker, ObservedChange, RxAppBuilder,
};
use ferroui_splat::{AppBuilder as SplatAppBuilder, AppLocator};
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

// Not ported: UseReactiveUI_ThrowsOnNullBuilder - argument validation (the builder is a value,
// never null).

#[test]
fn use_reactive_ui_after_platform_callback_builds_reactive_ui() {
    let _app = start();
    SplatAppBuilder::reset_builder_state_for_tests();
    let invoked = Rc::new(Cell::new(false));
    let seen = invoked.clone();

    StartupConfiguration::configure_reactive_ui(&move |_| seen.set(true));

    assert!(invoked.get());
    assert!(SplatAppBuilder::has_been_built());
}

#[test]
fn use_reactive_ui_after_platform_callback_when_already_built_returns_after_user_callback() {
    let _app = start();
    let invoked = Rc::new(Cell::new(false));
    let seen = invoked.clone();

    StartupConfiguration::configure_reactive_ui(&move |_| seen.set(true));

    assert!(invoked.get());
}

// Not ported: AppBuilderExtensions_IsStaticClass - reflection (the extensions are traits).
// Not ported: WithFerro_ThrowsOnNullBuilder - argument validation (the builder is a reference,
// never null).

/// Upstream checks the type of each service (`IsTypeOf<..>`); the traits of the services have no
/// `Any`, so the services are recognised by what they do: the fetcher gives a control view the
/// affinity of visuals, the hook gives an items control its template, and the view thread invoker
/// is the shared instance.
#[test]
fn with_ferro_registers_ferro_services() {
    let _app = start();
    reset_reactive_ui();
    let builder = RxAppBuilder::create_reactive_ui_builder();
    let _ = builder.with_ferro().build_app();

    let fetcher = AppLocator::get_service::<dyn IActivationForViewFetcher>().expect("an activation fetcher is registered");
    let control = ReactiveUserControlBase::new();
    let view = ReactiveViewFor::new(control);
    assert_eq!(fetcher.get_affinity_for_view(&*view), FerroActivationForViewFetcher::VISUAL_AFFINITY);

    let hook = AppLocator::get_service::<dyn IPropertyBindingHook>().expect("a binding hook is registered");
    let items = ListBox::new();
    let sender = FerroObjectHandles::handle_of(&items);
    let target: Rc<dyn Any> = sender.clone();
    let changes = move || vec![ObservedChange::new(sender.clone(), "ItemsSource", Rc::new(()) as Rc<dyn Any>)];
    assert!(hook.execute_hook(None, &target, &|| Vec::new(), &changes, BindingDirection::OneWay));
    assert!(items.item_template().is_some_and(|template| {
        std::ptr::addr_eq(Rc::as_ptr(&template), Rc::as_ptr(&AutoDataTemplateBindingHook::default_item_template()))
    }));

    assert!(AppLocator::get_service::<dyn ICreatesCommandBinding>().is_some());
    assert!(AppLocator::get_service::<dyn ICreatesObservableForProperty>().is_some());
    let invoker = AppLocator::get_service::<dyn IViewThreadInvoker>().expect("a view thread invoker is registered");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&invoker), Rc::as_ptr(&FerroViewThreadInvoker::instance())));
}

/// The error of a pipeline nothing handles reaches the unhandled-exception events of the
/// dispatcher as a failing job, without a panic (#567); an application's own handler, set
/// before `with_ferro`, is kept.
#[test]
fn with_ferro_raises_unhandled_pipeline_errors_on_the_dispatcher_without_a_panic() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_reactiveui::UnhandledError;
    use ferroui_system::ObservableError;
    use std::cell::RefCell;

    let _app = start();
    reset_reactive_ui();
    let builder = RxAppBuilder::create_reactive_ui_builder();
    let handler = builder.with_ferro().exception_handler().expect("with_ferro sets the handler");

    let dispatcher = Dispatcher::ui_thread();
    let received = Rc::new(RefCell::new(None::<UnhandledError>));
    let seen = received.clone();
    let _subscription = dispatcher.unhandled_exception(move |args| {
        *seen.borrow_mut() = args.error().and_then(|error| error.downcast_ref::<UnhandledError>()).cloned();
        args.set_handled(true);
    });
    let error: ObservableError = Rc::new(std::io::Error::other("the pipeline broke"));
    handler.on_next(error);
    assert!(received.borrow().is_none(), "raised from a job of the dispatcher, not in the pipeline");
    dispatcher.run_jobs(None);
    let received = received.take().expect("the dispatcher received the error");
    assert_eq!(received.inner_exception(), Some("the pipeline broke"));

    reset_reactive_ui();
    let own: Rc<dyn ferroui_system::IObserver<ObservableError>> = ferroui_reactive::Observer::create(|_| {});
    let builder = RxAppBuilder::create_reactive_ui_builder();
    builder.with_exception_handler(own.clone());
    let kept = builder.with_ferro().exception_handler().unwrap();
    assert!(Rc::ptr_eq(&kept, &own));
}
