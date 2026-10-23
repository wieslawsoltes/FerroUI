//! Behavioral ports of ServiceCollectionDescriptorExtensionsTests.cs (runtime v10.0.13).
use super::*;

#[test]
fn add_adds_descriptor_to_service_descriptors() {
    let d = ServiceDescriptor::instance(Rc::new(1usize));
    let mut s = ServiceCollection::new();
    s.add(d.clone());
    assert_eq!(s.len(), 1);
    assert_eq!(s.index_of(&d), Some(0));
}

#[test]
fn add_adds_multiple_descriptor_to_service_descriptors() {
    let a = descriptor(Transient, 1);
    let b = ServiceDescriptor::instance(Rc::new(1usize));
    let mut s = ServiceCollection::new();
    s.add(a.clone()).add(b.clone());
    assert_eq!(s.len(), 2);
    assert_eq!(s.index_of(&a), Some(0));
    assert_eq!(s.index_of(&b), Some(1));
}

#[test]
fn service_descriptor_to_string() {
    for lifetime in LIFETIMES {
        for key in [None, Some("key".into())] {
            let d = descriptor(lifetime, 1)
                .with_key(key.clone())
                .with_implementation::<Value>();
            let text = d.to_string();
            assert!(text.contains(std::any::type_name::<dyn Service>()));
            assert!(text.contains(&format!("{lifetime:?}")));
            assert!(text.contains(std::any::type_name::<Value>()));
            if key.is_some() {
                assert!(text.contains("key"));
            }
        }
    }
}
