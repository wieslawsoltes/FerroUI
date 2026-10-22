//! Behavioral ports of ServiceProviderValidationTests.cs (runtime v10.0.13).
use super::*;
fn chain(outer: ServiceLifetime) -> ServiceCollection {
    let mut s = ServiceCollection::new();
    s.add(
        ServiceDescriptor::new::<usize, _>(outer, |_| panic!("invalid graph"))
            .with_dependencies(vec![Dependency::required::<String>()]),
    );
    s.add(
        ServiceDescriptor::new::<String, _>(Singleton, |_| panic!("invalid graph"))
            .with_dependencies(vec![Dependency::required::<u8>()]),
    );
    s.add(ServiceDescriptor::new::<u8, _>(Scoped, |_| Ok(Rc::new(1))));
    s
}
fn scope_options() -> ServiceProviderOptions {
    ServiceProviderOptions {
        validate_scopes: true,
        validate_on_build: false,
    }
}

#[test]
fn get_service_throws_when_scoped_is_injected_into_singleton_through_singleton() {
    let p = chain(Singleton)
        .build_service_provider(scope_options())
        .unwrap();
    assert_eq!(
        error(p.get_service::<usize>()),
        ServiceError::ScopedInSingleton(ServiceIdentifier::of::<u8>())
    );
}

#[test]
fn get_service_throws_when_scoped_is_injected_into_singleton_through_singleton_and_scoped_while_in_scope(
) {
    let p = chain(Scoped)
        .build_service_provider(scope_options())
        .unwrap();
    let scope = p.create_scope().unwrap();
    assert_eq!(
        error(scope.service_provider().get_service::<usize>()),
        ServiceError::ScopedInSingleton(ServiceIdentifier::of::<u8>())
    );
}

#[test]
fn build_service_provider_any_key_service_key_with_strongly_typed_argument() {
    for validate_on_build in [false, true] {
        let mut s = ServiceCollection::new();
        s.add(
            ServiceDescriptor::from_factory(
                Transient,
                service_factory!(i32, |key: typed_key(i32)| Rc::new(key)),
            )
            .with_key(Some(ServiceKey::ANY)),
        );
        let p = s
            .build_service_provider(ServiceProviderOptions {
                validate_scopes: true,
                validate_on_build,
            })
            .unwrap();
        assert_eq!(
            *p.get_required_keyed_service::<i32>(Some(&ServiceKey::new(42i32)))
                .unwrap(),
            42
        );
        assert_eq!(
            error(p.get_keyed_service::<i32>(Some(&"Hello".into()))),
            ServiceError::TypeMismatch(ServiceType::of::<i32>())
        );
    }
}

#[test]
fn build_service_provider_any_key_service_key_with_object_typed_argument() {
    for validate_on_build in [false, true] {
        let mut s = ServiceCollection::new();
        s.add(
            ServiceDescriptor::from_factory(
                Transient,
                service_factory!(Option<ServiceKey>, |key: key()| Rc::new(key)),
            )
            .with_key(Some(ServiceKey::ANY)),
        );
        let p = s
            .build_service_provider(ServiceProviderOptions {
                validate_scopes: true,
                validate_on_build,
            })
            .unwrap();
        for key in [ServiceKey::new(42i32), "hello".into()] {
            assert_eq!(
                *p.get_required_keyed_service::<Option<ServiceKey>>(Some(&key))
                    .unwrap(),
                Some(key)
            );
        }
    }
}

#[test]
fn build_service_provider_service_key_with_object_typed_argument() {
    for validate_on_build in [false, true] {
        let mut s = ServiceCollection::new();
        for key in [ServiceKey::new(42i32), "hello".into()] {
            s.add(
                ServiceDescriptor::from_factory(
                    Transient,
                    service_factory!(Option<ServiceKey>, |key: key()| Rc::new(key)),
                )
                .with_key(Some(key)),
            );
        }
        let p = s
            .build_service_provider(ServiceProviderOptions {
                validate_scopes: true,
                validate_on_build,
            })
            .unwrap();
        for key in [ServiceKey::new(42i32), "hello".into()] {
            assert_eq!(
                *p.get_required_keyed_service::<Option<ServiceKey>>(Some(&key))
                    .unwrap(),
                Some(key)
            );
        }
        assert!(p
            .get_keyed_service::<Option<ServiceKey>>(Some(&ServiceKey::new(false)))
            .unwrap()
            .is_none());
    }
}

#[test]
fn build_service_provider_any_key_service_key_with_object_and_int_typed_arguments() {
    for validate_on_build in [false, true] {
        let mut s = ServiceCollection::new();
        s.add(
            ServiceDescriptor::from_factory(
                Transient,
                service_factory!(
                    (Option<ServiceKey>, i32),
                    |key: key(), int: typed_key(i32)| Rc::new((key, int))
                ),
            )
            .with_key(Some(ServiceKey::ANY)),
        );
        let p = s
            .build_service_provider(ServiceProviderOptions {
                validate_scopes: true,
                validate_on_build,
            })
            .unwrap();
        assert_eq!(
            *p.get_required_keyed_service::<(Option<ServiceKey>, i32)>(Some(&ServiceKey::new(
                42i32
            )))
            .unwrap(),
            (Some(ServiceKey::new(42i32)), 42)
        );
    }
}

#[test]
fn scope_validation_should_be_able_to_distingush_generic_collections_when_get_service_is_called_on_root(
) {
    let mut s = ServiceCollection::new();
    s.add_sequence([
        descriptor(Transient, 1),
        descriptor(Scoped, 2),
        ServiceDescriptor::instance(Rc::new(3usize)),
        ServiceDescriptor::instance(Rc::new(4usize)),
    ]);
    let p = s.build_service_provider(scope_options()).unwrap();
    assert!(matches!(
        p.get_services::<dyn Service>(),
        Err(ServiceError::ScopedFromRoot(_))
    ));
    assert_eq!(
        p.get_services::<usize>()
            .unwrap()
            .iter()
            .map(|v| **v)
            .collect::<Vec<_>>(),
        [3, 4]
    );
}

#[test]
fn get_service_throws_when_get_service_for_service_with_multiple_implementation_scopes_where_last_is_scoped(
) {
    let mut s = ServiceCollection::new();
    s.add_sequence([descriptor(Singleton, 1), descriptor(Scoped, 2)]);
    let p = s.build_service_provider(scope_options()).unwrap();
    assert!(matches!(
        p.get_services::<dyn Service>(),
        Err(ServiceError::ScopedFromRoot(_))
    ));
    assert!(matches!(
        p.get_service::<dyn Service>(),
        Err(ServiceError::ScopedFromRoot(_))
    ));
}

#[test]
fn get_service_does_not_throw_when_get_service_for_non_scoped_implementation_with_multiple_implementation_scopes_where_last_is_scoped(
) {
    let mut s = ServiceCollection::new();
    s.add_sequence([
        descriptor(Singleton, 1),
        descriptor(Scoped, 2),
        ServiceDescriptor::instance(Rc::new(Value(3))),
    ]);
    let p = s.build_service_provider(scope_options()).unwrap();
    assert!(matches!(
        p.get_services::<dyn Service>(),
        Err(ServiceError::ScopedFromRoot(_))
    ));
    assert_eq!(p.get_required_service::<Value>().unwrap().0, 3);
}

#[test]
fn build_service_provider_validate_on_build_throws_when_scoped_is_injected_into_singleton_cached_call_sites(
) {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Scoped, 1));
    s.add(ServiceDescriptor::from_factory(
        Scoped,
        service_factory!(usize, |_s: required(dyn Service)| Rc::new(1)),
    ));
    s.add(ServiceDescriptor::from_factory(
        Singleton,
        service_factory!(String, |_s: required(dyn Service)| Rc::new(String::new())),
    ));
    let e = error(s.build_service_provider(options()));
    assert_eq!(
        e,
        ServiceError::Validation(vec![ServiceError::ScopedInSingleton(
            ServiceIdentifier::of::<dyn Service>()
        )])
    );
}

#[test]
fn build_service_provider_validate_on_build_does_not_throw_cached_call_sites() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Scoped, 1));
    s.add(ServiceDescriptor::from_factory(
        Scoped,
        service_factory!(usize, |s: required(dyn Service)| Rc::new(s.value())),
    ));
    s.add(ServiceDescriptor::from_factory(
        Scoped,
        service_factory!(String, |s: required(dyn Service)| Rc::new(
            s.value().to_string()
        )),
    ));
    let p = checked(&s);
    let scope = p.create_scope().unwrap();
    assert_eq!(
        *scope
            .service_provider()
            .get_required_service::<usize>()
            .unwrap(),
        1
    );
    assert_eq!(
        &*scope
            .service_provider()
            .get_required_service::<String>()
            .unwrap(),
        "1"
    );
}
