//! ActivatorUtilitiesTests.cs and FromKeyedServices constructor cases.
use super::*;
#[test]
fn create_instance_uses_constructor_services_and_explicit_arguments() {
    let prefix = "value:".to_owned();
    let factory = service_factory!(String, |value: required(usize)| Rc::new(format!(
        "{prefix}{value}"
    )));
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(7usize));
    let p = s.build_service_provider(options()).unwrap();
    let factory = ActivatorUtilities::create_factory(factory);
    let a = ActivatorUtilities::create_instance(&p, &factory).unwrap();
    let b = ActivatorUtilities::create_instance(&p, &factory).unwrap();
    assert_eq!(&*a, "value:7");
    assert!(!Rc::ptr_eq(&a, &b));
}
#[test]
fn get_service_or_create_instance_prefers_registered_service() {
    let mut s = ServiceCollection::new();
    let value = Rc::new(7usize);
    s.add_singleton_instance(value.clone());
    let p = s.build_service_provider(options()).unwrap();
    let factory = ObjectFactory::new(vec![], |_, _| panic!("must not activate"));
    assert!(Rc::ptr_eq(
        &value,
        &ActivatorUtilities::get_service_or_create_instance(&p, &factory).unwrap()
    ));
    assert_eq!(
        *ActivatorUtilities::get_service_or_create_instance(
            &p,
            &service_factory!(i32, || Rc::new(9))
        )
        .unwrap(),
        9
    );
}
#[test]
fn optional_and_enumerable_parameters_can_be_empty() {
    let mut s = ServiceCollection::new();
    s.add(ServiceDescriptor::from_factory(
        ServiceLifetime::Singleton,
        service_factory!(usize, |a: optional(dyn Service), b: all(dyn Service)| {
            assert!(a.is_none());
            Rc::new(b.len())
        }),
    ));
    let p = s.build_service_provider(options()).unwrap();
    assert_eq!(*p.get_required_service::<usize>().unwrap(), 0);
}
#[test]
fn keyed_inherited_and_service_key_parameters_are_injected() {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(0usize));
    s.add_keyed_singleton_instance(Some("a".into()), Rc::new(1usize));
    s.add_keyed_singleton_instance(Some("b".into()), Rc::new(2usize));
    let factory = service_factory!(String, |a: keyed(usize,"a"), current: inherited(usize), key: key(), plain: required(usize)| Rc::new(format!("{}:{}:{}:{}",a,current,key.unwrap().downcast_ref::<String>().unwrap(),plain)));
    s.add(
        ServiceDescriptor::from_factory(ServiceLifetime::Scoped, factory)
            .with_key(Some("b".into())),
    );
    let p = s.build_service_provider(options()).unwrap();
    let scope = p.create_scope().unwrap();
    assert_eq!(
        &*scope
            .service_provider()
            .get_required_keyed_service::<String>(Some(&"b".into()))
            .unwrap(),
        "1:2:b:0"
    );
}
#[test]
fn keyed_macro_evaluates_the_key_once() {
    let count = Cell::new(0);
    let factory = service_factory!(usize, |v: keyed(usize,{ count.set(count.get()+1); "a" })| v);
    assert_eq!(count.get(), 1);
    let mut s = ServiceCollection::new();
    s.add_keyed_singleton_instance(Some("a".into()), Rc::new(3usize));
    let p = s.build_service_provider(options()).unwrap();
    assert_eq!(
        *ActivatorUtilities::create_instance(&p, &factory).unwrap(),
        3
    );
    assert_eq!(count.get(), 1);
}
#[test]
fn activator_does_not_capture_result_for_disposal() {
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    struct Owned(Rc<Cell<usize>>);
    impl Drop for Owned {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let p = ServiceCollection::new()
        .build_service_provider(options())
        .unwrap();
    let factory = service_factory!(Owned, || Rc::new(Owned(c.clone())));
    let value = ActivatorUtilities::create_instance(&p, &factory).unwrap();
    p.dispose().unwrap();
    assert_eq!(count.get(), 0);
    drop(value);
    assert_eq!(count.get(), 1);
    assert!(matches!(
        ActivatorUtilities::create_instance(&p, &factory),
        Err(ServiceError::Disposed)
    ));
}
#[test]
fn activation_defaults_errors_missing_parameters_and_transients() {
    let mut s = ServiceCollection::new();
    s.add_transient::<usize, _>(|_| Ok(Rc::new(1)));
    let p = s.build_service_provider(options()).unwrap();
    let fallback =
        ObjectFactory::<usize>::new(vec![], |_, _| panic!("registered transient should win"));
    assert!(!Rc::ptr_eq(
        &ActivatorUtilities::get_service_or_create_instance(&p, &fallback).unwrap(),
        &ActivatorUtilities::get_service_or_create_instance(&p, &fallback).unwrap()
    ));
    let optional = service_factory!(String, |value: optional(u32)| Rc::new(
        value.map_or(9, |n| *n).to_string()
    ));
    assert_eq!(
        &*ActivatorUtilities::create_instance(&p, &optional).unwrap(),
        "9"
    );
    let missing = service_factory!(String, |value: required(u32)| Rc::new(value.to_string()));
    assert!(matches!(
        ActivatorUtilities::create_instance(&p, &missing),
        Err(ServiceError::MissingService(_))
    ));
    let failure = ObjectFactory::<String>::new(vec![], |_, _| {
        Err(ServiceError::Factory("constructor failure".into()))
    });
    assert_eq!(
        ActivatorUtilities::create_instance(&p, &failure).unwrap_err(),
        ServiceError::Factory("constructor failure".into())
    );
}
