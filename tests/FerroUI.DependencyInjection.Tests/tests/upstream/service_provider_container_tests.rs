//! Behavioral ports of ServiceProviderContainerTests.cs (runtime v10.0.13).
use super::*;
#[derive(Default, Debug, PartialEq, Eq)]
struct Defaults {
    duration: std::time::Duration,
    bytes: [u8; 16],
    custom: (u32, bool),
}
fn mixed(lifetime: ServiceLifetime) -> ServiceCollection {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Singleton, 7));
    s.add(ServiceDescriptor::from_factory(
        lifetime,
        service_factory!(
            (Rc<dyn Service>, Defaults),
            |service: required(dyn Service), value: optional(Defaults)| Rc::new((
                service,
                value
                    .as_deref()
                    .map(|v| Defaults {
                        duration: v.duration,
                        bytes: v.bytes,
                        custom: v.custom
                    })
                    .unwrap_or_default()
            ))
        ),
    ));
    s
}
struct Outer(Rc<dyn Service>);
impl Service for Outer {
    fn value(&self) -> usize {
        self.0.value()
    }
}
fn struct_graph(lifetime: ServiceLifetime) -> ServiceCollection {
    let mut s = ServiceCollection::new();
    s.add(descriptor(lifetime, 42));
    s.add(ServiceDescriptor::from_factory(
        lifetime,
        service_factory!(Outer, |s: required(dyn Service)| Rc::new(Outer(s))),
    ));
    s
}

#[test]
fn rethrow_original_exception_from_constructor() {
    let mut s = ServiceCollection::new();
    s.add_transient::<usize, _>(|_| Err(ServiceError::Factory("empty constructor".into())));
    s.add(descriptor(Transient, 1));
    s.add_transient::<String, _>(|p| {
        p.get_required_service::<dyn Service>()?;
        Err(ServiceError::Factory("injected constructor".into()))
    });
    let p = build(&s);
    assert_eq!(
        error(p.get_service::<usize>()),
        ServiceError::Factory("empty constructor".into())
    );
    assert_eq!(
        error(p.get_service::<String>()),
        ServiceError::Factory("injected constructor".into())
    );
    assert!(p.get_service::<dyn Service>().unwrap().is_some());
}

#[test]
fn attempting_to_resolve_nonexistent_service_indirectly_throws() {
    let mut s = ServiceCollection::new();
    s.add(ServiceDescriptor::from_factory(
        Transient,
        service_factory!(usize, |s: required(dyn Service)| Rc::new(s.value())),
    ));
    missing::<dyn Service>(error(build(&s).get_service::<usize>()));
}

#[test]
fn attempting_to_i_enumerable_resolve_nonexistent_service_indirectly_throws() {
    let mut s = ServiceCollection::new();
    s.add(ServiceDescriptor::from_factory(
        Transient,
        service_factory!(usize, |s: required(dyn Service)| Rc::new(s.value())),
    ));
    missing::<dyn Service>(error(build(&s).get_services::<usize>()));
}

#[test]
fn creating_service_provider_with_unresolvable_types_throws() {
    // Rust cannot construct abstract/interface implementations; reject at the registration site.
    assert_compile_error(
        r#"use ferroui_dependency_injection::*;use std::rc::Rc;
trait Abstract {} fn test(){let _=ServiceDescriptor::new::<dyn Abstract,_>(ServiceLifetime::Transient,|_|Ok(Rc::new(Abstract{})));}"#,
        "E0574",
    );
}

#[test]
fn resolves_service_mixed_service_and_optional_struct_constructor_arguments() {
    let p = checked(&mixed(Singleton));
    let v = p
        .get_required_service::<(Rc<dyn Service>, Defaults)>()
        .unwrap();
    assert_eq!(v.0.value(), 7);
    assert_eq!(v.1, Defaults::default());
}

#[test]
fn resolves_service_mixed_service_and_optional_struct_constructor_arguments_reliably() {
    let p = checked(&mixed(Transient));
    for _ in 0..100 {
        let v = p
            .get_required_service::<(Rc<dyn Service>, Defaults)>()
            .unwrap();
        assert_eq!(v.0.value(), 7);
        assert_eq!(v.1, Defaults::default());
    }
}

#[test]
fn resolves_constant_value_type_services_correctly() {
    #[derive(Debug, PartialEq, Eq)]
    enum E {
        Hello,
    }
    for lifetime in LIFETIMES {
        let mut s = ServiceCollection::new();
        s.add(ServiceDescriptor::new::<i32, _>(lifetime, |_| {
            Ok(Rc::new(4))
        }));
        s.add(ServiceDescriptor::new::<Defaults, _>(lifetime, |_| {
            Ok(Rc::new(Defaults::default()))
        }));
        s.add(ServiceDescriptor::new::<E, _>(lifetime, |_| {
            Ok(Rc::new(E::Hello))
        }));
        for seconds in [0, 3723] {
            s.add(ServiceDescriptor::new::<std::time::Duration, _>(
                lifetime,
                move |_| Ok(Rc::new(std::time::Duration::from_secs(seconds))),
            ));
        }
        let p = build(&s);
        assert_eq!(*p.get_required_service::<i32>().unwrap(), 4);
        assert_eq!(
            *p.get_required_service::<Defaults>().unwrap(),
            Defaults::default()
        );
        assert_eq!(*p.get_required_service::<E>().unwrap(), E::Hello);
        assert_eq!(
            p.get_services::<std::time::Duration>()
                .unwrap()
                .iter()
                .map(|t| t.as_secs())
                .collect::<Vec<_>>(),
            [0, 3723]
        );
    }
}

#[test]
fn root_provider_dispose_prevents_service_resolution() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Singleton, 1));
    let p = build(&s);
    p.dispose().unwrap();
    assert_eq!(
        error(p.get_service::<dyn Service>()),
        ServiceError::Disposed
    );
}

#[test]
fn scope_dispose_prevents_service_resolution() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Scoped, 1));
    let p = checked(&s);
    let scope = p.create_scope().unwrap();
    scope.dispose().unwrap();
    assert_eq!(
        error(scope.service_provider().get_service::<dyn Service>()),
        ServiceError::Disposed
    );
    assert!(p.create_scope().is_ok());
}

#[test]
fn get_async_service_dispose_async_on_same_thread_throws_and_does_not_hang_and_dispose_async_gets_called(
) {
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let gate = TaskCompletionSource::new();
    let g = gate.clone();
    let mut s = ServiceCollection::new();
    s.add(
        ServiceDescriptor::new::<usize, _>(Singleton, |p| {
            p.dispose()?;
            Ok(Rc::new(1))
        })
        .on_dispose_async::<usize>(move |_| {
            c.set(c.get() + 1);
            g.task()
        }),
    );
    let p = build(&s);
    assert_eq!(error(p.get_service::<usize>()), ServiceError::Disposed);
    assert_eq!(count.get(), 1);
    gate.set_result(());
    assert!(gate.task().is_completed_successfully());
}

#[test]
fn add_disposables_and_async_disposables_dispose_async_all_disposed() {
    for delayed in [false, true] {
        let log = Rc::new(RefCell::new(vec![]));
        let gate = TaskCompletionSource::new();
        let mut s = ServiceCollection::new();
        let l = log.clone();
        s.add(
            ServiceDescriptor::new::<usize, _>(Singleton, |_| Ok(Rc::new(1)))
                .on_dispose::<usize>(move |_| l.borrow_mut().push(1)),
        );
        let l = log.clone();
        s.add(
            ServiceDescriptor::new::<u8, _>(Singleton, |_| Ok(Rc::new(2))).on_dispose_async::<u8>(
                move |_| {
                    l.borrow_mut().push(2);
                    Task::from_result(())
                },
            ),
        );
        let l = log.clone();
        let g = gate.clone();
        s.add(
            ServiceDescriptor::new::<u16, _>(Singleton, |_| Ok(Rc::new(3)))
                .on_dispose_async::<u16>(move |_| {
                    let l = l.clone();
                    let task = g.task();
                    Task::spawn_local(async move {
                        task.await?;
                        l.borrow_mut().push(3);
                        Ok(())
                    })
                }),
        );
        let p = build(&s);
        p.get_required_service::<usize>().unwrap();
        p.get_required_service::<u8>().unwrap();
        if delayed {
            p.get_required_service::<u16>().unwrap();
        }
        let done = p.dispose_async();
        assert_eq!(done.is_completed(), !delayed);
        gate.set_result(());
        assert!(done.is_completed_successfully());
        assert_eq!(
            &*log.borrow(),
            if delayed { &[3, 2, 1][..] } else { &[2, 1][..] }
        );
    }
}

#[test]
fn works_with_struct_services() {
    for lifetime in LIFETIMES {
        let mut s = struct_graph(lifetime);
        s.add(ServiceDescriptor::from_factory(
            lifetime,
            service_factory!(dyn Service, || Rc::new(Value(42))),
        ));
        let p = build(&s);
        assert_eq!(p.get_required_service::<Outer>().unwrap().value(), 42);
    }
}

#[test]
fn works_with_factory_struct_services() {
    for lifetime in LIFETIMES {
        let mut s = ServiceCollection::new();
        s.add(descriptor(lifetime, 17));
        assert_eq!(
            build(&s)
                .get_required_service::<dyn Service>()
                .unwrap()
                .value(),
            17
        );
    }
}

#[test]
fn works_with_factory_struct_services_as_dependencies() {
    for lifetime in LIFETIMES {
        let p = build(&struct_graph(lifetime));
        let service = p.get_required_service::<Outer>().unwrap();
        assert_eq!(service.0.value(), 42);
    }
}

#[test]
fn works_with_i_enumerable_struct_services() {
    for lifetime in LIFETIMES {
        let mut s = ServiceCollection::new();
        for i in 0..10 {
            s.add(descriptor(lifetime, i));
        }
        let values = build(&s).get_services::<dyn Service>().unwrap();
        assert_eq!(
            values.iter().map(|v| v.value()).collect::<Vec<_>>(),
            (0..10).collect::<Vec<_>>()
        );
    }
}

#[test]
fn works_with_wide_scoped_trees() {
    type Middle = Vec<Rc<dyn Service>>;
    type Top = Vec<Rc<Middle>>;
    let mut s = ServiceCollection::new();
    for i in 0..20 {
        s.add(descriptor(Scoped, i));
        s.add(ServiceDescriptor::from_factory(
            Scoped,
            service_factory!(Middle, |v: all(dyn Service)| Rc::new(v)),
        ));
        s.add(ServiceDescriptor::from_factory(
            Scoped,
            service_factory!(Top, |v: all(Middle)| Rc::new(v)),
        ));
    }
    let p = checked(&s);
    let scope = p.create_scope().unwrap();
    let values = scope.service_provider().get_services::<Top>().unwrap();
    assert_eq!(values.len(), 20);
    for top in &values {
        assert_eq!(top.len(), 20);
        for middle in top.iter() {
            assert_eq!(middle.len(), 20);
            assert!(Rc::ptr_eq(&middle[0], &values[0][0][0]));
        }
    }
}

#[test]
fn generic_i_enumerable_item_cached_in_the_right_slot() {
    struct Generic<T>(T);
    let mut s = ServiceCollection::new();
    s.add_singleton::<Generic<usize>, _>(|_| Ok(Rc::new(Generic(42))));
    s.add(descriptor(Singleton, 1));
    let p = build(&s);
    let single = p.get_required_service::<Generic<usize>>().unwrap();
    let many = p.get_services::<Generic<usize>>().unwrap();
    assert_eq!(many.len(), 1);
    assert_eq!(single.0, 42);
    assert!(Rc::ptr_eq(&single, &many[0]));
}

#[test]
fn provider_dispose_prefers_service_dispose() {
    let sync = Rc::new(Cell::new(0));
    let c = sync.clone();
    let mut s = ServiceCollection::new();
    s.add(
        descriptor(Transient, 1)
            .on_dispose::<dyn Service>(move |_| c.set(c.get() + 1))
            .on_dispose_async::<dyn Service>(|_| panic!("sync disposal must win")),
    );
    let p = build(&s);
    p.get_required_service::<dyn Service>().unwrap();
    p.dispose().unwrap();
    assert_eq!(sync.get(), 1);
}

#[test]
fn provider_scope_dispose_throws_when_only_dispose_async_implemented() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Transient, 1).on_dispose_async::<dyn Service>(|_| Task::from_result(())));
    let p = build(&s);
    let scope = p.create_scope().unwrap();
    scope
        .service_provider()
        .get_required_service::<dyn Service>()
        .unwrap();
    assert_eq!(scope.dispose(), Err(ServiceError::AsyncDisposalRequired));
    assert!(scope.dispose_async().is_completed_successfully());
}

#[test]
fn provider_async_scope_dispose_throws_when_only_dispose_async_implemented() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Transient, 1).on_dispose_async::<dyn Service>(|_| Task::from_result(())));
    let p = build(&s);
    let scope = p.create_async_scope().unwrap();
    scope
        .service_provider()
        .get_required_service::<dyn Service>()
        .unwrap();
    assert_eq!(scope.dispose(), Err(ServiceError::AsyncDisposalRequired));
    assert!(scope.dispose_async().is_completed_successfully());
}

#[test]
fn resolve_keyed_service_with_keyed_parameter_missing_registration_but_with_unkeyed_service() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Singleton, 1));
    s.add(ServiceDescriptor::from_factory(
        Singleton,
        service_factory!(usize,|v:keyed(dyn Service,"missing")|Rc::new(v.value())),
    ));
    assert_eq!(
        error(s.build_service_provider(options())),
        ServiceError::Validation(vec![ServiceError::MissingService(
            ServiceIdentifier::keyed::<dyn Service>(Some("missing".into()))
        )])
    );
}
