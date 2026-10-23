//! Behavioral ports of ServiceProviderServiceExtensionsTest.cs (runtime v10.0.13).
use super::*;
struct Alternate {
    values: Vec<Rc<dyn Service>>,
}
impl IServiceProvider for Alternate {
    fn get_service_erased(&self, id: &ServiceIdentifier) -> ServiceResult<Option<ErasedService>> {
        Ok(if id == &ServiceIdentifier::of::<dyn Service>() {
            self.values.last().cloned().map(erase_service)
        } else {
            None
        })
    }
    fn get_services_erased(&self, id: &ServiceIdentifier) -> ServiceResult<Vec<ErasedService>> {
        Ok(if id == &ServiceIdentifier::of::<dyn Service>() {
            self.values.iter().cloned().map(erase_service).collect()
        } else {
            vec![]
        })
    }
}
fn alternate(n: usize) -> Alternate {
    Alternate {
        values: (0..n)
            .map(|i| Rc::new(Value(i)) as Rc<dyn Service>)
            .collect(),
    }
}
struct Required;
impl IServiceProvider for Required {
    fn required_service_provider(&self) -> Option<&dyn ISupportRequiredService> {
        Some(self)
    }
    fn get_service_erased(&self, _: &ServiceIdentifier) -> ServiceResult<Option<ErasedService>> {
        panic!("must use required capability")
    }
    fn get_services_erased(&self, _: &ServiceIdentifier) -> ServiceResult<Vec<ErasedService>> {
        panic!("not used")
    }
}
impl ISupportRequiredService for Required {
    fn get_required_service_erased(&self, id: &ServiceIdentifier) -> ServiceResult<ErasedService> {
        if id == &ServiceIdentifier::of::<dyn Service>() {
            Ok(erase_service::<dyn Service>(Rc::new(Value(42))))
        } else {
            Err(ServiceError::Factory("custom required error".into()))
        }
    }
}

#[test]
fn get_service_returns_correct_service() {
    assert_eq!(
        alternate(1)
            .get_service::<dyn Service>()
            .unwrap()
            .unwrap()
            .value(),
        0
    );
}

#[test]
fn i_support_required_service_get_required_service_returns_correct_service() {
    let p: &dyn IServiceProvider = &Required;
    assert_eq!(p.get_required_service::<dyn Service>().unwrap().value(), 42);
}

#[test]
fn get_required_service_throws_when_no_service_registered() {
    missing::<dyn Service>(error(alternate(0).get_required_service::<dyn Service>()));
}

#[test]
fn i_support_required_service_get_required_service_throws_when_no_service_registered() {
    let p: &dyn IServiceProvider = &Required;
    assert_eq!(
        error(p.get_required_service::<usize>()),
        ServiceError::Factory("custom required error".into())
    );
}

#[test]
fn non_generic_get_required_service_throws_when_no_service_registered() {
    missing::<dyn Service>(error(
        alternate(0).required_service_erased(&ServiceIdentifier::of::<dyn Service>()),
    ));
}

#[test]
fn i_support_required_service_non_generic_get_required_service_throws_when_no_service_registered() {
    let p: &dyn IServiceProvider = &Required;
    assert_eq!(
        error(p.required_service_erased(&ServiceIdentifier::of::<usize>())),
        ServiceError::Factory("custom required error".into())
    );
}

#[test]
fn get_services_returns_all_services() {
    let v = alternate(2).get_services::<dyn Service>().unwrap();
    assert_eq!(v.iter().map(|s| s.value()).collect::<Vec<_>>(), [0, 1]);
}

#[test]
fn non_generic_get_services_returns_all_services() {
    let v = alternate(2)
        .get_services_erased(&ServiceIdentifier::of::<dyn Service>())
        .unwrap();
    assert_eq!(
        v.iter()
            .map(|s| unerase_service::<dyn Service>(s).unwrap().value())
            .collect::<Vec<_>>(),
        [0, 1]
    );
}

#[test]
fn get_services_returns_single_service() {
    let v = alternate(1).get_services::<dyn Service>().unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].value(), 0);
}

#[test]
fn non_generic_get_services_returns_single_service() {
    let v = alternate(1)
        .get_services_erased(&ServiceIdentifier::of::<dyn Service>())
        .unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(unerase_service::<dyn Service>(&v[0]).unwrap().value(), 0);
}

#[test]
fn get_services_returns_correct_types() {
    let mut s = ServiceCollection::new();
    s.add_sequence([
        descriptor(Transient, 1),
        descriptor(Transient, 2),
        ServiceDescriptor::instance(Rc::new(3usize)),
        ServiceDescriptor::instance(Rc::new(4usize)),
    ]);
    let v = build(&s)
        .get_services_erased(&ServiceIdentifier::of::<usize>())
        .unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(
        v.iter()
            .map(|x| *unerase_service::<usize>(x).unwrap())
            .collect::<Vec<_>>(),
        [3, 4]
    );
}

#[test]
fn get_services_returns_empty_array_when_no_services_available() {
    let values: Vec<Rc<dyn Service>> = alternate(0).get_services().unwrap();
    assert!(values.is_empty());
}

#[test]
fn non_generic_get_services_returns_empty_array_when_no_services_available() {
    assert!(alternate(0)
        .get_services_erased(&ServiceIdentifier::of::<dyn Service>())
        .unwrap()
        .is_empty());
}

#[test]
fn get_services_with_build_service_provider_returns_empty_list_when_no_services_available() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Transient, 1));
    s.add(ServiceDescriptor::enumerable_instance::<dyn Service>(
        vec![],
    ));
    let p = build(&s);
    assert!(p.get_services::<dyn Service>().unwrap().is_empty());
    assert!(p
        .get_required_service::<Vec<Rc<dyn Service>>>()
        .unwrap()
        .is_empty());
}

#[test]
fn non_generic_get_services_with_build_service_provider_returns_empty_list_when_no_services_available(
) {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Transient, 1));
    s.add(ServiceDescriptor::enumerable_instance::<dyn Service>(
        vec![],
    ));
    assert!(build(&s)
        .get_services_erased(&ServiceIdentifier::of::<dyn Service>())
        .unwrap()
        .is_empty());
}

#[test]
fn create_async_scope_returns_async_service_scope_wrapping_service_scope() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Scoped, 1));
    let p = checked(&s);
    let iface: &dyn IServiceProvider = &p;
    let scope = iface.create_async_scope().unwrap();
    assert_eq!(
        scope
            .service_provider()
            .get_required_service::<dyn Service>()
            .unwrap()
            .value(),
        1
    );
    assert!(scope.dispose_async().is_completed_successfully());
}

#[test]
fn create_async_scope_returns_async_service_scope_wrapping_service_scope_for_i_service_scope_factory(
) {
    let mut s = ServiceCollection::new();
    s.add(descriptor(Scoped, 1));
    let p = checked(&s);
    let f = p
        .get_required_service::<dyn IServiceScopeFactory>()
        .unwrap();
    let scope = f.create_async_scope().unwrap();
    assert_eq!(
        scope
            .service_provider()
            .get_required_service::<dyn Service>()
            .unwrap()
            .value(),
        1
    );
    assert!(scope.dispose_async().is_completed_successfully());
}

#[test]
fn explicit_enumerable_overrides_preserve_keys_identity_and_validation() {
    let first: Rc<dyn Service> = Rc::new(Value(10));
    let last: Rc<dyn Service> = Rc::new(Value(20));
    let keyed: Rc<dyn Service> = Rc::new(Value(30));
    let mut services = ServiceCollection::new();
    // The overridden scoped descriptor must not make a singleton's enumerable invalid.
    services.add(descriptor(Scoped, 1));
    services.add(ServiceDescriptor::enumerable_instance(vec![first]));
    services.add(ServiceDescriptor::enumerable_instance(vec![last.clone()]));
    services.add(
        ServiceDescriptor::enumerable_instance(vec![keyed.clone()]).with_key(Some("key".into())),
    );
    services.add(ServiceDescriptor::from_factory(
        Singleton,
        service_factory!(Rc<dyn Service>, |values: all(dyn Service)| Rc::new(
            values[0].clone()
        )),
    ));
    let provider = checked(&services);
    let values = provider.get_services::<dyn Service>().unwrap();
    assert_eq!(values.len(), 1);
    assert!(Rc::ptr_eq(&last, &values[0]));
    assert!(Rc::ptr_eq(
        &last,
        &provider.get_required_service::<Rc<dyn Service>>().unwrap()
    ));
    let values = provider
        .get_keyed_services::<dyn Service>(Some(&"key".into()))
        .unwrap();
    assert_eq!(values.len(), 1);
    assert!(Rc::ptr_eq(&keyed, &values[0]));
    provider.dispose().unwrap();
    assert_eq!(
        error(provider.get_services::<dyn Service>()),
        ServiceError::Disposed
    );
}
