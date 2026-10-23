//! Behavioral ports of ServiceCollectionKeyedServiceExtensionsTest.cs (runtime v10.0.13).
use super::*;
fn key() -> Option<ServiceKey> {
    Some("service".into())
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

#[test]
fn replace_replaces_first_service_with_matching_service_type() {
    let k: ServiceKey = "key1".into();
    let mut s = ServiceCollection::new();
    let a = descriptor(Transient, 1).with_key(Some(k.clone()));
    let b = descriptor(Transient, 2).with_key(Some(k.clone()));
    let c = descriptor(Singleton, 3).with_key(Some(k));
    let unrelated = descriptor(Transient, 4).with_key(Some("other".into()));
    s.add_sequence([a.clone(), b.clone(), unrelated.clone()]);
    s.replace(c.clone());
    assert_eq!(s.len(), 3);
    assert!(!s.contains(&a));
    assert_eq!(s.index_of(&b), Some(0));
    assert_eq!(s.index_of(&unrelated), Some(1));
    assert_eq!(s.index_of(&c), Some(2));
}

#[test]
fn replace_replaces_first_service_with_matching_service_type_when_key_is_int() {
    let k = ServiceKey::new(1i32);
    let mut s = ServiceCollection::new();
    let a = descriptor(Transient, 1).with_key(Some(k.clone()));
    let b = descriptor(Transient, 2).with_key(Some(k.clone()));
    let c = descriptor(Singleton, 3).with_key(Some(k));
    let unrelated = descriptor(Transient, 4).with_key(Some("other".into()));
    s.add_sequence([a.clone(), b.clone(), unrelated.clone()]);
    s.replace(c.clone());
    assert_eq!(s.len(), 3);
    assert!(!s.contains(&a));
    assert_eq!(s.index_of(&b), Some(0));
    assert_eq!(s.index_of(&unrelated), Some(1));
    assert_eq!(s.index_of(&c), Some(2));
}

#[test]
fn remove_all_removes_all_matching_services_when_key_is_enum() {
    #[derive(Debug, Hash, PartialEq, Eq)]
    enum K {
        First,
        Second,
    }
    let k1 = ServiceKey::new(K::First);
    let k2 = ServiceKey::new(K::Second);
    let a = descriptor(Transient, 1).with_key(Some(k1));
    let mut s = ServiceCollection::new();
    s.add_sequence([
        a.clone(),
        descriptor(Transient, 2).with_key(Some(k2.clone())),
        descriptor(Transient, 3).with_key(Some(k2.clone())),
    ]);
    s.remove_all_keyed::<dyn Service>(Some(k2));
    assert_eq!(s.len(), 1);
    assert!(s.contains(&a));
}

#[test]
fn remove_all_removes_all_matching_services_when_key_is_int() {
    let k1 = ServiceKey::new(1i32);
    let k2 = ServiceKey::new(2i32);
    let a = descriptor(Transient, 1).with_key(Some(k1));
    let mut s = ServiceCollection::new();
    s.add_sequence([
        a.clone(),
        descriptor(Transient, 2).with_key(Some(k2.clone())),
        descriptor(Transient, 3).with_key(Some(k2.clone())),
    ]);
    s.remove_all_keyed::<dyn Service>(Some(k2));
    assert_eq!(s.len(), 1);
    assert!(s.contains(&a));
}

#[test]
fn null_service_key_is_keyed_service_false() {
    for d in [
        descriptor(Transient, 1),
        ServiceDescriptor::instance(Rc::new(2usize)),
        ServiceDescriptor::from_factory(Scoped, service_factory!(usize, || Rc::new(3))),
    ] {
        assert!(!d.with_key(None).is_keyed_service());
    } // Rust exposes common typed metadata accessors rather than throwing keyed-only CLR properties.
}

#[test]
fn not_null_service_key_is_keyed_service_true() {
    for d in [
        descriptor(Transient, 1),
        ServiceDescriptor::instance(Rc::new(2usize)),
        ServiceDescriptor::from_factory(Scoped, service_factory!(usize, || Rc::new(3))),
    ] {
        let d = d.with_key(Some("key".into()));
        assert!(d.is_keyed_service());
        assert_eq!(d.service_key(), Some(&"key".into()));
    }
}
