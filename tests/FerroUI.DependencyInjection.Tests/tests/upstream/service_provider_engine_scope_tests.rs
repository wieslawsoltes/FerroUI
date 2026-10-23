//! Behavioral ports of ServiceProviderEngineScopeTests.cs (runtime v10.0.13).
use super::*;

#[test]
fn double_dispose_works() {
    let calls = Rc::new(Cell::new(0));
    let c = calls.clone();
    let mut s = ServiceCollection::new();
    s.add(descriptor(Scoped, 1).on_dispose::<dyn Service>(move |_| c.set(c.get() + 1)));
    let p = checked(&s);
    let scope = p.create_scope().unwrap();
    scope
        .service_provider()
        .get_required_service::<dyn Service>()
        .unwrap();
    scope.dispose().unwrap();
    scope.dispose().unwrap();
    assert_eq!(calls.get(), 1);
}

#[test]
fn root_engine_scope_dispose_test() {
    let p = build(&ServiceCollection::new());
    let root = p.get_required_service::<dyn IServiceProvider>().unwrap();
    root.disposable().expect("root is disposable").dispose();
    assert_eq!(error(p.get_service::<usize>()), ServiceError::Disposed);
}

#[test]
fn service_provider_engine_scope_implements_all_service_provider_interfaces() {
    let p = build(&ServiceCollection::new());
    let scope = p.create_scope().unwrap();
    let p = scope.service_provider();
    assert!(p.get_service::<dyn IServiceProvider>().unwrap().is_some());
    assert!(p
        .get_service::<dyn IKeyedServiceProvider>()
        .unwrap()
        .is_some());
    assert!(p
        .get_service::<dyn IServiceScopeFactory>()
        .unwrap()
        .is_some());
    assert!(p
        .get_service::<dyn IServiceProviderIsService>()
        .unwrap()
        .is_some());
    assert!(p
        .get_service::<dyn IServiceProviderIsKeyedService>()
        .unwrap()
        .is_some());
}
