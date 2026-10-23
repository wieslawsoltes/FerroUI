//! Behavioral ports of CallSiteTests.cs (runtime v10.0.13).
use super::*;

#[test]
fn does_not_throw_when_service_is_used_as_enumerable_and_not_in_one_call_site() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Transient, 1));
    s.add(ServiceDescriptor::from_factory(
        Transient,
        service_factory!(
            (Rc<dyn Service>, Vec<Rc<dyn Service>>),
            |one: required(dyn Service), many: all(dyn Service)| Rc::new((one, many))
        ),
    ));
    let v = checked(&s)
        .get_required_service::<(Rc<dyn Service>, Vec<Rc<dyn Service>>)>()
        .unwrap();
    assert_eq!(v.0.value(), 1);
    assert_eq!(v.1.len(), 1);
    assert_eq!(v.1[0].value(), 1);
    assert!(!Rc::ptr_eq(&v.0, &v.1[0]));
}

#[test]
fn no_service_callsite_default_value_null_does_not_throw() {
    let mut s = ServiceCollection::new();
    s.add_transient::<u8, _>(|_| Ok(Rc::new(1)));
    let f = service_factory!(
        (Rc<u8>, Option<Rc<String>>),
        |g: required(u8), missing: optional(String)| Rc::new((g, missing))
    );
    let v = ActivatorUtilities::create_instance(&checked(&s), &f).unwrap();
    assert_eq!(*v.0, 1);
    assert!(v.1.is_none());
}

#[test]
fn service_call_site_should_have_key_when_resolving_keyed_service() {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(None::<ServiceKey>));
    let d = ServiceDescriptor::from_factory(
        Transient,
        service_factory!(Option<ServiceKey>, |key: key()| Rc::new(key)),
    )
    .with_key(Some("someKey".into()));
    assert_eq!(d.service_key(), Some(&"someKey".into()));
    s.add(d);
    let p = checked(&s);
    assert_eq!(
        *p.get_required_keyed_service::<Option<ServiceKey>>(Some(&"someKey".into()))
            .unwrap(),
        Some("someKey".into())
    );
    assert!(p
        .get_required_service::<Option<ServiceKey>>()
        .unwrap()
        .is_none());
}
