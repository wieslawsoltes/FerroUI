//! `ReactiveUIBuilderViewLocatorTests` (`ReactiveUI.Builder.Tests/ReactiveUIBuilderViewLocatorTests.cs`).
//!
//! `IsAssignableTo<DefaultViewLocator>()` and `IsTypeOf<ConfiguredView>()` are downcasts of the
//! resolved services.

use super::executors::reset_only;
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `ConfiguredViewModel`.
#[derive(Default)]
pub struct ConfiguredViewModel {
    reactive: ReactiveObject,
}
reactive_object!(ConfiguredViewModel);

/// `MappedViewModel`.
#[derive(Default)]
pub struct MappedViewModel {
    reactive: ReactiveObject,
}
reactive_object!(MappedViewModel);

/// `ConfiguredView`.
#[derive(Default)]
pub struct ConfiguredView {
    view_model: RefCell<Option<Rc<ConfiguredViewModel>>>,
}
view_for!(ConfiguredView, ConfiguredViewModel, view_model);

/// `MappedView`.
#[derive(Default)]
pub struct MappedView {
    view_model: RefCell<Option<Rc<MappedViewModel>>>,
}
view_for!(MappedView, MappedViewModel, view_model);

fn registered_view_locator() -> Option<Rc<dyn IViewLocator>> {
    AppLocator::get_service::<dyn IViewLocator>()
}

fn as_default_view_locator(locator: Rc<dyn IViewLocator>) -> Option<Rc<DefaultViewLocator>> {
    (locator as Rc<dyn Any>).downcast::<DefaultViewLocator>().ok()
}

#[test]
fn configure_view_locator_configures_one_locator_instance() {
    reset_only(|| {
        let configure_calls = Rc::new(Cell::new(0));
        let calls = configure_calls.clone();
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.with_core_services();
        let _ = builder.configure_view_locator(move |_| calls.set(calls.get() + 1)).build_app();

        let first = registered_view_locator().unwrap();
        let second = registered_view_locator().unwrap();

        assert!(as_default_view_locator(first.clone()).is_some());
        assert!(Rc::ptr_eq(&second, &first));
        assert_eq!(configure_calls.get(), 1);
    });
}

#[test]
fn configure_view_locator_keeps_mappings_added_by_register_views() {
    reset_only(|| {
        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.with_core_services();
        let _ = builder
            .configure_view_locator(|locator| locator.map::<ConfiguredViewModel, ConfiguredView>(None))
            .register_views(|views| {
                let _ = views.map::<MappedViewModel, MappedView>(None);
            })
            .build_app();

        let locator = as_default_view_locator(registered_view_locator().unwrap()).unwrap();

        let configured = locator.resolve_view_for::<ConfiguredViewModel>(None).unwrap();
        let mapped = locator.resolve_view_for::<MappedViewModel>(None).unwrap();
        assert!((&*configured as &dyn Any).is::<ConfiguredView>());
        assert!((&*mapped as &dyn Any).is::<MappedView>());
    });
}
