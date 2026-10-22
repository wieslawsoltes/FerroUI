//! KeyedDependencyInjectionSpecificationTests.cs, including AnyKey cache identity.
use super::*;
#[test]
fn resolve_null_keyed_service() {
    let mut s = ServiceCollection::new();
    s.add_keyed_singleton::<dyn Service, _>(None, |_, k| {
        assert!(k.is_none());
        Ok(Rc::new(Value(1)))
    });
    let p = s.build_service_provider(options()).unwrap();
    assert!(Rc::ptr_eq(
        &p.get_required_service::<dyn Service>().unwrap(),
        &p.get_required_keyed_service::<dyn Service>(None).unwrap()
    ));
}
#[test]
fn resolve_keyed_service() {
    let mut s = ServiceCollection::new();
    for (key, n) in [(None, 0), (Some(""), 1), (Some("a"), 2), (Some("b"), 3)] {
        s.add(descriptor(ServiceLifetime::Singleton, n).with_key(key.map(Into::into)));
    }
    let p = s.build_service_provider(options()).unwrap();
    for (key, n) in [(None, 0), (Some(""), 1), (Some("a"), 2), (Some("b"), 3)] {
        assert_eq!(
            p.get_required_keyed_service::<dyn Service>(key.map(ServiceKey::from).as_ref())
                .unwrap()
                .value(),
            n
        );
    }
    assert!(p
        .get_keyed_service::<dyn Service>(Some(&"missing".into()))
        .unwrap()
        .is_none());
}
#[test]
fn keys_use_type_and_value_equality() {
    #[derive(Debug, PartialEq, Eq, Hash)]
    struct Key(u32);
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Singleton, 1).with_key(Some(ServiceKey::new(Key(3)))));
    let p = s.build_service_provider(options()).unwrap();
    assert!(p
        .get_keyed_service::<dyn Service>(Some(&ServiceKey::new(Key(3))))
        .unwrap()
        .is_some());
    assert!(p
        .get_keyed_service::<dyn Service>(Some(&ServiceKey::new(3u32)))
        .unwrap()
        .is_none());
}
#[test]
fn resolve_keyed_service_singleton_factory_with_any_key() {
    let mut s = ServiceCollection::new();
    s.add_keyed_singleton::<String, _>(Some(ServiceKey::ANY), |_, k| {
        Ok(Rc::new(
            k.unwrap().downcast_ref::<String>().unwrap().clone(),
        ))
    });
    let p = s.build_service_provider(options()).unwrap();
    let a = "a".into();
    let b = "b".into();
    let av = p.get_required_keyed_service::<String>(Some(&a)).unwrap();
    let bv = p.get_required_keyed_service::<String>(Some(&b)).unwrap();
    assert_eq!(&*av, "a");
    assert_eq!(&*bv, "b");
    assert!(!Rc::ptr_eq(&av, &bv));
    assert!(Rc::ptr_eq(
        &av,
        &p.get_required_keyed_service::<String>(Some(&a)).unwrap()
    ));
    assert!(p.get_service::<String>().unwrap().is_none());
    assert!(p.is_keyed_service(ServiceType::of::<String>(), Some(&ServiceKey::ANY)));
    assert!(p.is_keyed_service(ServiceType::of::<String>(), Some(&a)));
    assert!(matches!(
        p.get_keyed_service::<String>(Some(&ServiceKey::ANY)),
        Err(ServiceError::AnyKeySingleResolution)
    ));
}
#[test]
fn resolve_keyed_services_any_key_consistency_with_any_key_registration() {
    for enumerate_first in [false, true] {
        let mut s = ServiceCollection::new();
        for (key, n) in [(Some("a"), 1), (None, 0), (Some("b"), 2), (Some("a"), 3)] {
            s.add(descriptor(ServiceLifetime::Singleton, n).with_key(key.map(Into::into)));
        }
        s.add(descriptor(ServiceLifetime::Singleton, 9).with_key(Some(ServiceKey::ANY)));
        let p = s.build_service_provider(options()).unwrap();
        let before = if enumerate_first {
            p.get_keyed_services::<dyn Service>(Some(&ServiceKey::ANY))
                .unwrap()
        } else {
            Vec::new()
        };
        let a = p
            .get_required_keyed_service::<dyn Service>(Some(&"a".into()))
            .unwrap();
        let b = p
            .get_required_keyed_service::<dyn Service>(Some(&"b".into()))
            .unwrap();
        let all = if enumerate_first {
            before
        } else {
            p.get_keyed_services::<dyn Service>(Some(&ServiceKey::ANY))
                .unwrap()
        };
        assert_eq!(
            all.iter().map(|v| v.value()).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(Rc::ptr_eq(&all[1], &b));
        assert!(Rc::ptr_eq(&all[2], &a));
        assert!(p
            .get_keyed_services::<dyn Service>(Some(&"missing".into()))
            .unwrap()
            .is_empty());
        assert_eq!(
            p.get_required_keyed_service::<dyn Service>(Some(&"missing".into()))
                .unwrap()
                .value(),
            9
        );
    }
}
#[test]
fn keyed_scoped_services_share_only_within_scope_and_key() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1).with_key(Some(ServiceKey::ANY)));
    let p = s.build_service_provider(options()).unwrap();
    let a = p.create_scope().unwrap();
    let b = p.create_scope().unwrap();
    let av = a
        .service_provider()
        .get_required_keyed_service::<dyn Service>(Some(&"a".into()))
        .unwrap();
    assert!(Rc::ptr_eq(
        &av,
        &a.service_provider()
            .get_required_keyed_service::<dyn Service>(Some(&"a".into()))
            .unwrap()
    ));
    assert!(!Rc::ptr_eq(
        &av,
        &a.service_provider()
            .get_required_keyed_service::<dyn Service>(Some(&"b".into()))
            .unwrap()
    ));
    assert!(!Rc::ptr_eq(
        &av,
        &b.service_provider()
            .get_required_keyed_service::<dyn Service>(Some(&"a".into()))
            .unwrap()
    ));
}
#[test]
fn keyed_provider_trait_object_supports_typed_resolution() {
    let mut s = ServiceCollection::new();
    s.add_keyed_singleton_instance(Some("a".into()), Rc::new(1usize));
    let p = s.build_service_provider(options()).unwrap();
    let keyed = p
        .get_required_service::<dyn IKeyedServiceProvider>()
        .unwrap();
    assert_eq!(
        *keyed
            .get_required_keyed_service::<usize>(Some(&"a".into()))
            .unwrap(),
        1
    );
}
#[test]
fn any_key_constructor_inherits_concrete_request_key() {
    let mut s = ServiceCollection::new();
    s.add_keyed_singleton::<usize, _>(Some(ServiceKey::ANY), |_, k| {
        Ok(Rc::new(*k.unwrap().downcast_ref::<usize>().unwrap()))
    });
    s.add(
        ServiceDescriptor::from_factory(
            ServiceLifetime::Singleton,
            service_factory!(String, |v: inherited(usize)| Rc::new(v.to_string())),
        )
        .with_key(Some(ServiceKey::ANY)),
    );
    let p = s.build_service_provider(options()).unwrap();
    assert_eq!(
        &*p.get_required_keyed_service::<String>(Some(&ServiceKey::new(7usize)))
            .unwrap(),
        "7"
    );
    assert_eq!(
        &*p.get_required_keyed_service::<String>(Some(&ServiceKey::new(9usize)))
            .unwrap(),
        "9"
    );
}
#[test]
fn keyed_lifetimes_work_through_scopes_and_injected_providers() {
    for lifetime in [
        ServiceLifetime::Singleton,
        ServiceLifetime::Scoped,
        ServiceLifetime::Transient,
    ] {
        let mut s = ServiceCollection::new();
        s.add(descriptor(lifetime, 7).with_key(Some("a".into())));
        let p = s.build_service_provider(options()).unwrap();
        let scope = p.create_scope().unwrap();
        let injected = scope
            .service_provider()
            .get_required_service::<dyn IServiceProvider>()
            .unwrap();
        let first = injected
            .get_required_keyed_service::<dyn Service>(Some(&"a".into()))
            .unwrap();
        let second = scope
            .service_provider()
            .get_required_keyed_service::<dyn Service>(Some(&"a".into()))
            .unwrap();
        assert_eq!(first.value(), 7);
        assert_eq!(
            Rc::ptr_eq(&first, &second),
            lifetime != ServiceLifetime::Transient
        );
        assert!(injected
            .get_keyed_service::<dyn Service>(Some(&"unknown".into()))
            .unwrap()
            .is_none());
        assert!(matches!(
            injected.get_required_keyed_service::<dyn Service>(Some(&"unknown".into())),
            Err(ServiceError::MissingService(_))
        ));
        scope.dispose().unwrap();
        p.dispose().unwrap();
    }
}
#[test]
fn keyed_parameters_do_not_fall_back_to_unkeyed_services() {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(1usize));
    s.add(ServiceDescriptor::from_factory(
        ServiceLifetime::Transient,
        service_factory!(String,|v:keyed(usize,"missing")|Rc::new(v.to_string())),
    ));
    assert!(
        matches!(s.build_service_provider(options()),Err(ServiceError::Validation(v)) if matches!(&v[0],ServiceError::MissingService(id) if id.service_key==Some("missing".into())))
    );
}
#[test]
fn closed_generic_types_and_key_probes_remain_distinct() {
    let mut s = ServiceCollection::new();
    s.add_keyed_singleton_instance(Some("a".into()), Rc::new(vec![1usize]));
    s.add_singleton_instance(Rc::new(vec!["a".to_owned()]));
    let p = s.build_service_provider(options()).unwrap();
    assert_eq!(
        &*p.get_required_keyed_service::<Vec<usize>>(Some(&"a".into()))
            .unwrap(),
        &[1]
    );
    assert!(!p.is_service(ServiceType::of::<Vec<usize>>()));
    assert!(p.is_service(ServiceType::of::<Vec<String>>()));
    let probe = p
        .get_required_service::<dyn IServiceProviderIsKeyedService>()
        .unwrap();
    assert!(probe.is_keyed_service(ServiceType::of::<Vec<usize>>(), Some(&"a".into())));
    assert!(!probe.is_keyed_service(ServiceType::of::<Vec<String>>(), Some(&"a".into())));
    assert!(probe.is_keyed_service(ServiceType::of::<Vec<String>>(), None));
}
#[test]
fn keyed_instances_and_catch_all_instances_preserve_identity_and_order() {
    let first: Rc<dyn Service> = Rc::new(Value(1));
    let second: Rc<dyn Service> = Rc::new(Value(2));
    let fallback: Rc<dyn Service> = Rc::new(Value(3));
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(first.clone());
    s.add_keyed_singleton_instance(Some("a".into()), first.clone());
    s.add_keyed_singleton_instance(Some("a".into()), second.clone());
    s.add_keyed_singleton_instance(Some("b".into()), first.clone());
    s.add_keyed_singleton_instance(Some(ServiceKey::ANY), fallback.clone());
    let p = s.build_service_provider(options()).unwrap();
    let all = p
        .get_keyed_services::<dyn Service>(Some(&ServiceKey::ANY))
        .unwrap();
    assert_eq!(all.len(), 3);
    assert!(Rc::ptr_eq(&all[0], &first));
    assert!(Rc::ptr_eq(&all[1], &second));
    assert!(Rc::ptr_eq(&all[2], &first));
    let a = p
        .get_keyed_services::<dyn Service>(Some(&"a".into()))
        .unwrap();
    assert_eq!(a.len(), 2);
    assert!(Rc::ptr_eq(
        &a[1],
        &p.get_required_keyed_service::<dyn Service>(Some(&"a".into()))
            .unwrap()
    ));
    for key in ["c", "d"] {
        assert!(Rc::ptr_eq(
            &fallback,
            &p.get_required_keyed_service::<dyn Service>(Some(&key.into()))
                .unwrap()
        ));
    }
    assert!(p
        .get_keyed_services::<dyn Service>(Some(&"c".into()))
        .unwrap()
        .is_empty());
    assert!(Rc::ptr_eq(
        &first,
        &p.get_required_service::<dyn Service>().unwrap()
    ));
}
#[test]
fn catch_all_transient_only_matches_the_requested_service_type() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Transient, 1).with_key(Some(ServiceKey::ANY)));
    let p = s.build_service_provider(options()).unwrap();
    let first = p
        .get_required_keyed_service::<dyn Service>(Some(&"a".into()))
        .unwrap();
    let second = p
        .get_required_keyed_service::<dyn Service>(Some(&"a".into()))
        .unwrap();
    assert!(!Rc::ptr_eq(&first, &second));
    assert!(p
        .get_keyed_service::<usize>(Some(&"a".into()))
        .unwrap()
        .is_none());
}
#[test]
fn missing_keyed_parameters_fail_in_either_position_and_can_use_explicit_defaults() {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(1usize));
    let p = s.build_service_provider(options()).unwrap();
    let first = service_factory!(usize,|a:keyed(usize,"missing"),b:required(usize)|Rc::new(*a+*b));
    let second = service_factory!(usize,|a:required(usize),b:keyed(usize,"missing")|Rc::new(*a+*b));
    for factory in [first, second] {
        assert!(matches!(
            ActivatorUtilities::create_instance(&p, &factory),
            Err(ServiceError::MissingService(_))
        ));
    }
    let default = ObjectFactory::new(
        vec![Dependency::keyed::<usize>(Some("missing".into())).optional()],
        |p, _| {
            Ok(p.get_keyed_service(Some(&"missing".into()))?
                .unwrap_or_else(|| Rc::new(9usize)))
        },
    );
    assert_eq!(
        *ActivatorUtilities::create_instance(&p, &default).unwrap(),
        9
    );
}
#[test]
fn service_key_parameter_supports_unkeyed_activation() {
    let p = ServiceCollection::new()
        .build_service_provider(options())
        .unwrap();
    let factory = service_factory!(Option<ServiceKey>, |key: key()| Rc::new(key));
    assert!(ActivatorUtilities::create_instance(&p, &factory)
        .unwrap()
        .is_none());
    assert_eq!(
        *ActivatorUtilities::create_keyed_instance(&p, Some(&"a".into()), &factory).unwrap(),
        Some("a".into())
    );
}
