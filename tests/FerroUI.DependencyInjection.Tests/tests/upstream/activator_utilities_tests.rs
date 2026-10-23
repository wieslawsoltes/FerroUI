//! Behavioral ports of ActivatorUtilitiesTests.cs (runtime v10.0.13).
use super::*;
struct A;
struct B;
struct C;
struct S;
struct Z;
fn services() -> ServiceCollection {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(A))
        .add_singleton_instance(Rc::new(B))
        .add_singleton_instance(Rc::new(C))
        .add_singleton_instance(Rc::new(S))
        .add_singleton_instance(Rc::new(Z));
    s
}

#[test]
fn create_instance_one_ctor_is_registered_creates_instance_successfully() {
    let mut s = ServiceCollection::new();
    s.add_scoped::<A, _>(|_| Ok(Rc::new(A)));
    let p = checked(&s);
    let scope = p.create_scope().unwrap();
    let p = scope.service_provider();
    let f = service_factory!(Rc<A>, |a: required(A)| Rc::new(a));
    let v = ActivatorUtilities::create_instance(p, &f).unwrap();
    assert!(Rc::ptr_eq(&*v, &p.get_required_service::<A>().unwrap()));
}

#[test]
fn create_instance_badly_configured_i_service_provider_is_service_properly_creates_unambiguous_instance(
) {
    struct BadProbe;
    impl IServiceProviderIsService for BadProbe {
        fn is_service(&self, _: ServiceType) -> bool {
            false
        }
    }
    struct Alternate(ServiceProvider);
    impl IServiceProvider for Alternate {
        fn get_service_erased(
            &self,
            id: &ServiceIdentifier,
        ) -> ServiceResult<Option<ErasedService>> {
            if id == &ServiceIdentifier::of::<dyn IServiceProviderIsService>() {
                Ok(Some(erase_service::<dyn IServiceProviderIsService>(
                    Rc::new(BadProbe),
                )))
            } else {
                self.0.get_service_erased(id)
            }
        }
        fn get_services_erased(&self, id: &ServiceIdentifier) -> ServiceResult<Vec<ErasedService>> {
            self.0.get_services_erased(id)
        }
    }
    let mut s = services();
    s.add_singleton_instance::<dyn IServiceProviderIsService>(Rc::new(BadProbe));
    let p = Alternate(build(&s));
    assert!(!p
        .get_required_service::<dyn IServiceProviderIsService>()
        .unwrap()
        .is_service(ServiceType::of::<A>()));
    let f = service_factory!(Rc<A>, |a: required(A)| Rc::new(a));
    let v = ActivatorUtilities::create_instance(&p, &f).unwrap();
    assert!(Rc::ptr_eq(&*v, &p.get_required_service::<A>().unwrap()));
    // A typed constructor needs no heuristic probe even when a custom probe reports false.
}

#[test]
fn create_instance_different_orders_creates_instance_successfully() {
    let mut s = ServiceCollection::new();
    s.add_scoped::<S, _>(|_| Ok(Rc::new(S)));
    let p = build(&s);
    let b = Rc::new(B);
    let c = Rc::new(C);
    let (b1, c1) = (b.clone(), c.clone());
    let f = service_factory!(
        (Option<Rc<A>>, Rc<B>, Rc<C>, Rc<S>),
        |a: optional(A), s: required(S)| Rc::new((a, b1.clone(), c1.clone(), s))
    );
    let (b2, c2) = (b.clone(), c.clone());
    let g = service_factory!(
        (Option<Rc<A>>, Rc<B>, Rc<C>, Rc<S>),
        |s: required(S), a: optional(A)| Rc::new((a, b2.clone(), c2.clone(), s))
    );
    for f in [f, g] {
        let v = ActivatorUtilities::create_instance(&p, &f).unwrap();
        assert!(v.0.is_none());
        assert!(Rc::ptr_eq(&b, &v.1));
        assert!(Rc::ptr_eq(&c, &v.2));
        assert!(Rc::ptr_eq(&v.3, &p.get_required_service::<S>().unwrap()));
    }
}

#[test]
fn create_instance_null_instance_handles_bad_input_with_invalid_operation_exception() {
    // Rust rejects a null explicit constructor argument before execution.
    assert_compile_error(
        r#"use ferroui_dependency_injection::*; use std::rc::Rc;
struct B; fn construct(_:Rc<B>)->Rc<usize>{Rc::new(1)}
fn test(){let null_b:Option<Rc<B>>=None;let _=service_factory!(usize,||construct(null_b.clone()));}"#,
        "E0308",
    );
}

#[test]
fn create_factory_creates_factory_method_4_types_3_injected() {
    let p = build(&services());
    let b = Rc::new(B);
    let captured = b.clone();
    let f = ActivatorUtilities::create_factory(service_factory!(
        (Rc<A>, Rc<B>, Rc<C>, Rc<S>),
        |a: required(A), c: required(C), s: required(S)| Rc::new((a, captured.clone(), c, s))
    ));
    for _ in 0..2 {
        let v = ActivatorUtilities::create_instance(&p, &f).unwrap();
        assert!(Rc::ptr_eq(&v.0, &p.get_required_service::<A>().unwrap()));
        assert!(Rc::ptr_eq(&v.1, &b));
        assert!(Rc::ptr_eq(&v.2, &p.get_required_service::<C>().unwrap()));
        assert!(Rc::ptr_eq(&v.3, &p.get_required_service::<S>().unwrap()));
    }
}

#[test]
fn create_factory_creates_factory_method_5_types_5_injected() {
    let p = build(&services());
    let f = ActivatorUtilities::create_factory(service_factory!(
        (Rc<A>, Rc<B>, Rc<C>, Rc<S>, Rc<Z>),
        |a: required(A), b: required(B), c: required(C), s: required(S), z: required(Z)| Rc::new((
            a, b, c, s, z
        ))
    ));
    let v = ActivatorUtilities::create_instance(&p, &f).unwrap();
    assert!(Rc::ptr_eq(&v.0, &p.get_required_service::<A>().unwrap()));
    assert!(Rc::ptr_eq(&v.1, &p.get_required_service::<B>().unwrap()));
    assert!(Rc::ptr_eq(&v.2, &p.get_required_service::<C>().unwrap()));
    assert!(Rc::ptr_eq(&v.3, &p.get_required_service::<S>().unwrap()));
    assert!(Rc::ptr_eq(&v.4, &p.get_required_service::<Z>().unwrap()));
}

#[test]
fn create_factory_creates_factory_method_keyed_params() {
    let mut s = services();
    let b = Rc::new(B);
    let c = Rc::new(C);
    s.add_keyed_singleton_instance(Some("b".into()), b.clone())
        .add_keyed_singleton_instance(Some("c".into()), c.clone());
    let p = build(&s);
    let f = service_factory!((Rc<A>,Rc<B>,Rc<C>),|a:required(A),b:keyed(B,"b"),c:keyed(C,"c")|Rc::new((a,b,c)));
    let v = ActivatorUtilities::create_instance(&p, &f).unwrap();
    assert!(Rc::ptr_eq(&v.0, &p.get_required_service::<A>().unwrap()));
    assert!(Rc::ptr_eq(&v.1, &b));
    assert!(Rc::ptr_eq(&v.2, &c));
}

#[test]
fn create_factory_creates_factory_method_keyed_params_5_types() {
    let mut s = services();
    let b = Rc::new(B);
    let c = Rc::new(C);
    s.add_keyed_singleton_instance(Some("b".into()), b.clone())
        .add_keyed_singleton_instance(Some("c".into()), c.clone());
    let p = build(&s);
    let f = service_factory!((Rc<A>,Rc<B>,Rc<C>,Rc<S>,Rc<Z>),|a:required(A),b:keyed(B,"b"),c:keyed(C,"c"),s:required(S),z:required(Z)|Rc::new((a,b,c,s,z)));
    let v = ActivatorUtilities::create_instance(&p, &f).unwrap();
    assert!(Rc::ptr_eq(&v.0, &p.get_required_service::<A>().unwrap()));
    assert!(Rc::ptr_eq(&v.1, &b));
    assert!(Rc::ptr_eq(&v.2, &c));
    assert!(Rc::ptr_eq(&v.3, &p.get_required_service::<S>().unwrap()));
    assert!(Rc::ptr_eq(&v.4, &p.get_required_service::<Z>().unwrap()));
}

#[test]
fn create_factory_creates_factory_method_keyed_params_value_types() {
    #[derive(Debug, Hash, PartialEq, Eq)]
    enum K {
        A,
    }
    let keys = [
        ServiceKey::new(u32::MAX),
        ServiceKey::new(i32::MAX),
        ServiceKey::new(u64::MAX),
        ServiceKey::new(i64::MAX),
        ServiceKey::new(K::A),
    ];
    let mut s = ServiceCollection::new();
    for (i, k) in keys.iter().enumerate() {
        s.add_keyed_singleton_instance(Some(k.clone()), Rc::new(i));
    }
    let f = service_factory!(
        [usize; 5],
        |b: keyed(usize, ServiceKey::new(u32::MAX)),
         c: keyed(usize, ServiceKey::new(i32::MAX)),
         s: keyed(usize, ServiceKey::new(u64::MAX)),
         y: keyed(usize, ServiceKey::new(i64::MAX)),
         z: keyed(usize, ServiceKey::new(K::A))| Rc::new([*b, *c, *s, *y, *z])
    );
    assert_eq!(
        *ActivatorUtilities::create_instance(&build(&s), &f).unwrap(),
        [0, 1, 2, 3, 4]
    );
}

#[test]
fn create_factory_creates_factory_method_keyed_params_1_injected() {
    let mut s = ServiceCollection::new();
    let a = Rc::new(A);
    let b = Rc::new(B);
    let c = Rc::new(C);
    s.add_keyed_singleton_instance(Some("b".into()), b.clone())
        .add_keyed_singleton_instance(Some("c".into()), c.clone());
    let captured = a.clone();
    let f = service_factory!((Rc<A>,Rc<B>,Rc<C>),|b:keyed(B,"b"),c:keyed(C,"c")|Rc::new((captured.clone(),b,c)));
    let v = ActivatorUtilities::create_instance(&build(&s), &f).unwrap();
    assert!(Rc::ptr_eq(&v.0, &a));
    assert!(Rc::ptr_eq(&v.1, &b));
    assert!(Rc::ptr_eq(&v.2, &c));
}
