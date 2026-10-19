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
