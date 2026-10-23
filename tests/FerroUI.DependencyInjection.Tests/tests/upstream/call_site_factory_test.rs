//! Behavioral ports of ServiceLookup/CallSiteFactoryTest.cs (runtime v10.0.13).
use super::*;
struct Node(Rc<usize>);
struct Branch(Rc<Node>);

#[test]
fn get_service_factory_call_site_transient_does_not_fail() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Transient, 1))
        .add_transient::<Value, _>(|_| Ok(Rc::new(Value(2))));
    let p = checked(&s);
    for _ in 0..50 {
        let a = p.get_required_service::<dyn Service>().unwrap();
        let b = p.get_required_service::<dyn Service>().unwrap();
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!(a.value(), 1);
        assert_eq!(p.get_required_service::<Value>().unwrap().0, 2);
    }
}

#[test]
fn create_call_site_creates_instance_call_site_if_type_has_default_or_public_parameterless_constructor(
) {
    let d = ServiceDescriptor::from_factory(Transient, service_factory!(usize, || Rc::new(0)));
    assert!(d.dependencies().is_empty());
    assert_eq!(d.lifetime(), Transient);
    let mut s = ServiceCollection::new();
    s.add(d);
    let p = checked(&s);
    assert_eq!(*p.get_required_service::<usize>().unwrap(), 0);
}

#[test]
fn create_call_site_creates_constructor_call_site_if_type_has_constructor_with_injectable_parameters(
) {
    let d = ServiceDescriptor::from_factory(
        Transient,
        service_factory!(Node, |v: required(usize)| Rc::new(Node(v))),
    );
    assert_eq!(d.dependencies().len(), 1);
    assert_eq!(
        d.dependencies()[0].identifier,
        ServiceIdentifier::of::<usize>()
    );
    let mut s = ServiceCollection::new();
    s.add(d).add_singleton_instance(Rc::new(42usize));
    assert_eq!(*checked(&s).get_required_service::<Node>().unwrap().0, 42);
}

#[test]
fn create_call_site_creates_constructor_with_enumerable_parameters() {
    let d = ServiceDescriptor::from_factory(
        Transient,
        service_factory!(
            (Vec<Rc<usize>>, Vec<Rc<String>>),
            |v: all(usize), other: all(String)| Rc::new((v, other))
        ),
    );
    assert!(d
        .dependencies()
        .iter()
        .all(|d| d.kind == DependencyKind::Enumerable));
    let mut s = ServiceCollection::new();
    s.add(d).add_singleton_instance(Rc::new(42usize));
    let v = checked(&s)
        .get_required_service::<(Vec<Rc<usize>>, Vec<Rc<String>>)>()
        .unwrap();
    assert_eq!(*v.0[0], 42);
    assert!(v.1.is_empty());
}

#[test]
fn create_call_site_returns_matching_types_that_match_correct_constraints() {
    // Closed generic registrations are checked by Rust's bounds, replacing MakeGenericType constraints.
    trait Generic<T> {
        fn value(&self) -> &T;
    }
    struct Plain<T>(T);
    struct DefaultBound<T: Default>(T);
    struct CopyBound<T: Copy>(T);
    impl<T> Generic<T> for Plain<T> {
        fn value(&self) -> &T {
            &self.0
        }
    }
    impl<T: Default> Generic<T> for DefaultBound<T> {
        fn value(&self) -> &T {
            &self.0
        }
    }
    impl<T: Copy> Generic<T> for CopyBound<T> {
        fn value(&self) -> &T {
            &self.0
        }
    }
    let mut s = ServiceCollection::new();
    s.add_singleton_instance::<dyn Generic<i32>>(Rc::new(Plain(1)))
        .add_singleton_instance::<dyn Generic<i32>>(Rc::new(DefaultBound(2)))
        .add_singleton_instance::<dyn Generic<i32>>(Rc::new(CopyBound(3)));
    s.add_singleton_instance::<dyn Generic<String>>(Rc::new(Plain("a".into())))
        .add_singleton_instance::<dyn Generic<String>>(Rc::new(DefaultBound("b".into())));
    let p = checked(&s);
    assert_eq!(
        p.get_services::<dyn Generic<i32>>()
            .unwrap()
            .iter()
            .map(|v| *v.value())
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert_eq!(
        p.get_services::<dyn Generic<String>>()
            .unwrap()
            .iter()
            .map(|v| v.value().clone())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}

#[test]
fn create_call_site_considers_constructors_with_default_values() {
    for present in [false, true] {
        let d = ServiceDescriptor::from_factory(
            Transient,
            service_factory!((usize, u8), |a: required(usize), b: optional(u8)| Rc::new(
                (*a, b.as_deref().copied().unwrap_or(7))
            )),
        );
        assert_eq!(d.dependencies()[1].kind, DependencyKind::Optional);
        let mut s = ServiceCollection::new();
        s.add(d).add_singleton_instance(Rc::new(42usize));
        if present {
            s.add_singleton_instance(Rc::new(9u8));
        }
        assert_eq!(
            *checked(&s).get_required_service::<(usize, u8)>().unwrap(),
            (42, if present { 9 } else { 7 })
        );
    }
}

#[test]
fn create_call_site_throws_if_type_has_single_constructor_with_unresolvable_parameters() {
    let mut s = ServiceCollection::new();
    s.add(ServiceDescriptor::from_factory(
        Transient,
        service_factory!(Node, |v: required(usize)| Rc::new(Node(v))),
    ));
    missing::<usize>(error(build(&s).get_service::<Node>()));
}

#[test]
fn get_slot_tests() {
    for n in 1..=5 {
        let mut s = ServiceCollection::new();
        for i in 0..n {
            s.add(descriptor(Singleton, i));
        }
        let p = checked(&s);
        let single = p.get_required_service::<dyn Service>().unwrap();
        let values = p.get_services::<dyn Service>().unwrap();
        let again = p.get_services::<dyn Service>().unwrap();
        assert!(Rc::ptr_eq(&single, &values[n - 1]));
        for (i, value) in values.iter().enumerate() {
            assert_eq!(value.value(), i);
            assert!(Rc::ptr_eq(value, &again[i]));
            for earlier in &values[..i] {
                assert!(!Rc::ptr_eq(value, earlier));
            }
        }
    }
}

#[test]
fn create_call_site_throws_if_type_has_no_constructur_with_resolvable_parameters() {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(1u8));
    s.add_singleton_instance(Rc::new("unrelated".to_owned()));
    s.add(ServiceDescriptor::from_factory(
        Transient,
        service_factory!(Node, |v: required(usize)| Rc::new(Node(v))),
    ));
    assert_eq!(
        error(s.build_service_provider(options())),
        ServiceError::Validation(vec![ServiceError::MissingService(ServiceIdentifier::of::<
            usize,
        >())])
    );
}

#[test]
fn create_call_site_enumberable_cached_at_lowest_level() {
    for a in LIFETIMES {
        for b in LIFETIMES {
            let mut s = ServiceCollection::new();
            s.add(descriptor(a, 1)).add(descriptor(b, 2));
            let p = checked(&s);
            let one = p.create_scope().unwrap();
            let two = p.create_scope().unwrap();
            let first = one
                .service_provider()
                .get_services::<dyn Service>()
                .unwrap();
            let second = one
                .service_provider()
                .get_services::<dyn Service>()
                .unwrap();
            let other = two
                .service_provider()
                .get_services::<dyn Service>()
                .unwrap();
            for (i, lifetime) in [a, b].iter().enumerate() {
                assert_eq!(Rc::ptr_eq(&first[i], &second[i]), *lifetime != Transient);
                assert_eq!(Rc::ptr_eq(&first[i], &other[i]), *lifetime == Singleton);
            }
        }
    }
}

#[test]
fn call_sites_are_unique_per_service_type_and_slot() {
    // Internal call-site identity is represented by immutable descriptor slots; verify their
    // observable cache identities throughout the connected graph on the supported local thread.
    for _ in 0..100 {
        let mut s = ServiceCollection::new();
        s.add_singleton::<usize, _>(|_| Ok(Rc::new(3)));
        s.add(ServiceDescriptor::from_factory(
            Singleton,
            service_factory!(Node, |v: required(usize)| Rc::new(Node(v))),
        ));
        s.add(ServiceDescriptor::from_factory(
            Singleton,
            service_factory!(Branch, |v: required(Node)| Rc::new(Branch(v))),
        ));
        s.add(ServiceDescriptor::from_factory(
            Singleton,
            service_factory!(Rc<usize>, |v: required(usize)| Rc::new(v)),
        ));
        s.add(ServiceDescriptor::from_factory(
            Singleton,
            service_factory!(Rc<Node>, |v: required(Node)| Rc::new(v)),
        ));
        let p = checked(&s);
        let branch = p.get_required_service::<Branch>().unwrap();
        let node = p.get_required_service::<Node>().unwrap();
        let leaf = p.get_required_service::<usize>().unwrap();
        assert!(Rc::ptr_eq(&branch.0, &node));
        assert!(Rc::ptr_eq(&node.0, &leaf));
        assert!(Rc::ptr_eq(
            &*p.get_required_service::<Rc<usize>>().unwrap(),
            &leaf
        ));
        assert!(Rc::ptr_eq(
            &*p.get_required_service::<Rc<Node>>().unwrap(),
            &node
        ));
    }
}
