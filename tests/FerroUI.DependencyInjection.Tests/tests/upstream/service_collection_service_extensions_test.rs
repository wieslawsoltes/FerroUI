//! Behavioral ports of ServiceCollectionServiceExtensionsTest.cs (runtime v10.0.13).
use super::*;
fn key() -> Option<ServiceKey> {
    None
}

#[test]
fn add_with_type_adds_service_with_right_lifecyle() {
    for lifetime in LIFETIMES {
        let mut s = ServiceCollection::new();
        s.add(
            ServiceDescriptor::from_factory(
                lifetime,
                service_factory!(dyn Service, || Rc::new(Value(9))),
            )
            .with_key(key())
            .with_implementation::<Value>(),
        );
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].service_type(), ServiceType::of::<dyn Service>());
        assert_eq!(s[0].implementation_type(), Some(ServiceType::of::<Value>()));
        assert_eq!(s[0].service_key(), key().as_ref());
        assert_eq!(s[0].lifetime(), lifetime);
    }
}

#[test]
fn add_with_factory_adds_service_with_right_lifecyle() {
    for lifetime in LIFETIMES {
        let mut s = ServiceCollection::new();
        s.add(
            ServiceDescriptor::keyed::<dyn Service, _>(lifetime, key(), |_, _| {
                Ok(Rc::new(Value(8)))
            })
            .with_implementation::<Value>(),
        );
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].lifetime(), lifetime);
        assert_eq!(s[0].service_type(), ServiceType::of::<dyn Service>());
        assert_eq!(s[0].service_key(), key().as_ref());
        assert!(s[0].implementation_factory().is_some());
        assert_eq!(s[0].implementation_type(), Some(ServiceType::of::<Value>()));
        assert_eq!(
            build(&s)
                .get_required_keyed_service::<dyn Service>(key().as_ref())
                .unwrap()
                .value(),
            8
        );
    }
}

#[test]
fn add_singleton_adds_with_singleton_lifecycle() {
    let instance: Rc<dyn Service> = Rc::new(Value(12));
    let mut s = ServiceCollection::new();
    s.add_keyed_singleton_instance(key(), instance.clone());
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].lifetime(), Singleton);
    assert_eq!(s[0].service_type(), ServiceType::of::<dyn Service>());
    assert_eq!(s[0].service_key(), key().as_ref());
    assert!(Rc::ptr_eq(
        &instance,
        &unerase_service::<dyn Service>(&s[0].implementation_instance().unwrap()).unwrap()
    ));
}

#[test]
fn try_add_enumerable_throws_when_adding_indistinguishable_implementation_type() {
    for d in [
        descriptor(Transient, 1).with_implementation::<dyn Service>(),
        descriptor(Transient, 1).with_implementation::<dyn std::any::Any>(),
        descriptor(Transient, 1),
    ] {
        let mut s = ServiceCollection::new();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            s.try_add_enumerable(d.with_key(key()));
        }))
        .is_err());
        assert!(s.is_empty());
    }
}

#[test]
fn add_sequence_adds_services_to_collection() {
    let a = descriptor(Transient, 1).with_key(key());
    let b = ServiceDescriptor::instance(Rc::new(2usize)).with_key(key());
    let mut s = ServiceCollection::new();
    s.add_sequence([a.clone(), b.clone()]);
    assert_eq!(s.len(), 2);
    assert_eq!(s.index_of(&a), Some(0));
    assert_eq!(s.index_of(&b), Some(1));
}

#[test]
fn replace_adds_service_if_service_type_is_not_registered() {
    let a = descriptor(Transient, 1).with_key(key());
    let b = ServiceDescriptor::instance(Rc::new(2usize)).with_key(key());
    let mut s = ServiceCollection::new();
    s.add(a.clone()).replace(b.clone());
    assert_eq!(s.len(), 2);
    assert_eq!(s.index_of(&a), Some(0));
    assert_eq!(s.index_of(&b), Some(1));
}
