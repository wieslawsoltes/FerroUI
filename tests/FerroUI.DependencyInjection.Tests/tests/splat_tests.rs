use super::*;
use ferroui_dependency_injection::splat::MicrosoftDependencyResolver;
use ferroui_splat::{MutableDependencyResolverExt, ReadonlyDependencyResolverExt};
#[test]
fn resolver_registrations_use_provider_and_keyed_contracts() {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(5usize));
    let r = MicrosoftDependencyResolver::new(s);
    assert_eq!(
        *ReadonlyDependencyResolverExt::get_service::<usize>(&r).unwrap(),
        5
    );
    r.register_with_contract::<usize>(|| Some(Rc::new(7)), Some("a"));
    r.register_with_contract::<usize>(|| Some(Rc::new(8)), Some(""));
    assert_eq!(*r.get_service_with_contract::<usize>(Some("a")).unwrap(), 7);
    assert_eq!(*r.get_service_with_contract::<usize>(Some("")).unwrap(), 8);
    assert_eq!(
        *r.service_provider()
            .unwrap()
            .get_required_keyed_service::<usize>(Some(&"a".into()))
            .unwrap(),
        7
    );
    assert_eq!(
        *ReadonlyDependencyResolverExt::get_service::<usize>(&r).unwrap(),
        5
    );
}
#[test]
fn mutation_disposes_old_provider_and_unregisters_only_requested_contract() {
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let mut s = ServiceCollection::new();
    s.add(
        ServiceDescriptor::new::<usize, _>(ServiceLifetime::Singleton, |_| Ok(Rc::new(1)))
            .on_dispose::<usize>(move |_| c.set(c.get() + 1)),
    );
    let r = MicrosoftDependencyResolver::new(s);
    let old = r.service_provider().unwrap();
    old.get_required_service::<usize>().unwrap();
    r.register_with_contract::<usize>(|| Some(Rc::new(2)), Some("a"));
    assert!(old.is_disposed());
    assert_eq!(count.get(), 1);
    r.register_with_contract::<usize>(|| Some(Rc::new(3)), Some("a"));
    r.unregister_current_with_contract::<usize>(Some("a"));
    assert_eq!(*r.get_service_with_contract::<usize>(Some("a")).unwrap(), 2);
    r.unregister_all_with_contract::<usize>(Some("a"));
    assert!(r.get_service_with_contract::<usize>(Some("a")).is_none());
    assert!(r.has_registration::<usize>());
}
#[test]
fn built_provider_is_immutable_and_supports_splat_trait_object() {
    let p = ServiceCollection::new()
        .build_service_provider(options())
        .unwrap();
    let r = Rc::new(MicrosoftDependencyResolver::from_provider(p));
    let erased: Rc<dyn ferroui_splat::IDependencyResolver> = r.clone();
    assert!(!erased.has_registration::<usize>());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || r.register_constant(Rc::new(1usize))
    ))
    .is_err());
    r.dispose().unwrap();
    assert!(r.service_provider().is_err());
}
#[test]
fn null_factories_are_none_and_omitted_from_enumeration() {
    let r = MicrosoftDependencyResolver::default();
    r.register::<usize>(|| Some(Rc::new(1)));
    r.register::<usize>(|| None);
    assert!(ReadonlyDependencyResolverExt::get_service::<usize>(&r).is_none());
    assert_eq!(
        ReadonlyDependencyResolverExt::get_services::<usize>(&r).len(),
        1
    );
}
#[test]
fn locator_initialization_and_resolution_use_adapter() {
    let old = ferroui_splat::Locator::get_locator();
    let r = MicrosoftDependencyResolver::use_splat(ServiceCollection::new());
    r.register_constant(Rc::new(17usize));
    assert_eq!(
        *ferroui_splat::Locator::current()
            .get_service::<usize>()
            .unwrap(),
        17
    );
    assert!(ferroui_splat::Locator::current()
        .get_service::<dyn ferroui_splat::ILogManager>()
        .is_some());
    ferroui_splat::Locator::set_locator(old);
    r.dispose().unwrap();
}
#[test]
fn reactiveui_registers_and_resolves_its_services_through_di() {
    use ferroui_reactiveui::{
        DependencyResolverRegistrar, IActivationForViewFetcher, IViewLocator,
        IWantsToRegisterStuff, Registrations,
    };
    let r = Rc::new(MicrosoftDependencyResolver::default());
    let registrar = DependencyResolverRegistrar::new(r.clone());
    Registrations.register(&registrar);
    let view = r.get_service::<dyn IViewLocator>().unwrap();
    let activation = r.get_service::<dyn IActivationForViewFetcher>().unwrap();
    let provider = r.service_provider().unwrap();
    assert!(Rc::ptr_eq(
        &view,
        &provider.get_required_service::<dyn IViewLocator>().unwrap()
    ));
    assert!(Rc::ptr_eq(
        &activation,
        &provider
            .get_required_service::<dyn IActivationForViewFetcher>()
            .unwrap()
    ));
    r.dispose().unwrap();
}
#[test]
fn failed_sync_rebuild_preserves_provider_for_async_cleanup() {
    use ferroui_system::threading::tasks::Task;
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let mut s = ServiceCollection::new();
    s.add(
        ServiceDescriptor::new::<usize, _>(ServiceLifetime::Singleton, |_| Ok(Rc::new(1)))
            .on_dispose_async::<usize>(move |_| {
                c.set(c.get() + 1);
                Task::from_result(())
            }),
    );
    let r = MicrosoftDependencyResolver::new(s);
    r.get_service::<usize>().unwrap();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || r.register_constant(Rc::new(2u32))
    ))
    .is_err());
    assert!(r.dispose_async().is_completed_successfully());
    assert_eq!(count.get(), 1);
}
#[test]
fn lazy_singletons_survive_rebuilds_and_dispose_with_the_resolver() {
    struct Counted(Rc<Cell<usize>>);
    impl ferroui_system::IDisposable for Counted {
        fn dispose(&self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let created = Rc::new(Cell::new(0));
    let disposed = Rc::new(Cell::new(0));
    let r = MicrosoftDependencyResolver::default();
    let (c, d) = (created.clone(), disposed.clone());
    r.register_lazy_singleton_disposable::<Counted>(move || {
        c.set(c.get() + 1);
        Some(Rc::new(Counted(d.clone())))
    });
    let first = r.get_service::<Counted>().unwrap();
    r.register_constant(Rc::new(1usize));
    let second = r.get_service::<Counted>().unwrap();
    assert!(Rc::ptr_eq(&first, &second));
    assert_eq!((created.get(), disposed.get()), (1, 0));
    r.dispose().unwrap();
    assert_eq!(disposed.get(), 1);
}
#[test]
fn updating_with_the_current_provider_keeps_it() {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(3usize));
    let r = MicrosoftDependencyResolver::new(s);
    let current = r.service_provider().unwrap();
    r.update_provider(current.clone()).unwrap();
    assert!(!current.is_disposed());
    assert_eq!(
        *ReadonlyDependencyResolverExt::get_service::<usize>(&r).unwrap(),
        3
    );
    r.dispose().unwrap();
    assert!(current.is_disposed());
}
