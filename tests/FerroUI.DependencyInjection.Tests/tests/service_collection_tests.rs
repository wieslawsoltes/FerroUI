use super::*;
#[test]
fn try_add_uses_type_and_key() {
    let mut s = ServiceCollection::new();
    s.try_add(descriptor(ServiceLifetime::Transient, 1));
    s.try_add(descriptor(ServiceLifetime::Transient, 2));
    s.try_add(descriptor(ServiceLifetime::Transient, 3).with_key(Some("a".into())));
    assert_eq!(s.len(), 2);
}
#[test]
fn replace_removes_first_match_and_appends() {
    let mut s = ServiceCollection::new();
    for n in 0..3 {
        s.add(descriptor(ServiceLifetime::Transient, n));
    }
    s.replace(descriptor(ServiceLifetime::Transient, 9));
    let p = s.build_service_provider(options()).unwrap();
    assert_eq!(
        p.get_services::<dyn Service>()
            .unwrap()
            .iter()
            .map(|v| v.value())
            .collect::<Vec<_>>(),
        vec![1, 2, 9]
    );
}
#[test]
fn try_add_enumerable_distinguishes_implementation_and_key() {
    let mut s = ServiceCollection::new();
    for _ in 0..2 {
        s.try_add_enumerable(
            descriptor(ServiceLifetime::Transient, 1).with_implementation::<Value>(),
        );
    }
    s.try_add_enumerable(descriptor(ServiceLifetime::Transient, 2).with_implementation::<usize>());
    s.try_add_enumerable(
        descriptor(ServiceLifetime::Transient, 3)
            .with_implementation::<Value>()
            .with_key(Some("a".into())),
    );
    assert_eq!(s.len(), 3);
}
#[test]
fn remove_all_only_removes_matching_type_and_key() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Transient, 1));
    s.add(descriptor(ServiceLifetime::Transient, 2).with_key(Some("a".into())));
    s.remove_all::<dyn Service>();
    assert_eq!(s.len(), 1);
    s.remove_all_keyed::<dyn Service>(Some("a".into()));
    assert!(s.is_empty());
}
#[test]
fn read_only_collection_rejects_mutation_but_can_build() {
    let mut s = ServiceCollection::new();
    s.make_read_only();
    assert!(s.is_read_only());
    assert!(s.build_service_provider(options()).is_ok());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.clear())).is_err());
}
#[test]
fn collection_identity_indexing_and_copy_preserve_descriptors() {
    let first = descriptor(ServiceLifetime::Transient, 1);
    let second = descriptor(ServiceLifetime::Transient, 2);
    let mut s = ServiceCollection::new();
    s.add(first.clone()).add(second.clone());
    assert!(s.contains(&first));
    assert_eq!(s.index_of(&second), Some(1));
    assert!(!s.contains(&descriptor(ServiceLifetime::Transient, 1)));
    let mut copy = vec![second.clone(); 3];
    s.copy_to(&mut copy, 1);
    assert_eq!(s.index_of(&copy[1]), Some(0));
    s[0] = second.clone();
    assert!(!s.contains(&first));
    assert!(s.remove(&second));
    assert_eq!(s.len(), 1);
    assert_eq!((&s).into_iter().count(), 1);
}
#[test]
fn default_provider_factory_applies_options_without_activating_services() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1));
    let factory = DefaultServiceProviderFactory::new(options());
    let p = factory
        .create_service_provider(factory.create_builder(s))
        .unwrap();
    assert!(matches!(
        p.get_service::<dyn Service>(),
        Err(ServiceError::ScopedFromRoot(_))
    ));
}
