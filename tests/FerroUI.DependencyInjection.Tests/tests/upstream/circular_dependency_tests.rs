//! Behavioral ports of CircularDependencyTests.cs (runtime v10.0.13).
use super::*;
struct Generic<T>(std::marker::PhantomData<T>);
fn edge<A: 'static, B: ?Sized + 'static>() -> ServiceDescriptor {
    ServiceDescriptor::new::<A, _>(Transient, |_| panic!("validation precedes activation"))
        .with_dependencies(vec![Dependency::required::<B>()])
}
fn cycle<T: ?Sized + 'static>(p: &ServiceProvider, expected: Vec<ServiceIdentifier>) {
    assert_eq!(
        error(p.get_required_service::<T>()),
        ServiceError::CircularDependency(expected)
    );
}

#[test]
fn self_circular_dependency_generic_direct() {
    let mut s = ServiceCollection::new();
    s.add(edge::<Generic<String>, Generic<String>>());
    cycle::<Generic<String>>(
        &build(&s),
        vec![
            ServiceIdentifier::of::<Generic<String>>(),
            ServiceIdentifier::of::<Generic<String>>(),
        ],
    );
}

#[test]
fn self_circular_dependency_generic_indirect() {
    let mut s = ServiceCollection::new();
    s.add(edge::<Generic<i32>, Generic<String>>())
        .add(edge::<Generic<String>, Generic<String>>());
    cycle::<Generic<i32>>(
        &build(&s),
        vec![
            ServiceIdentifier::of::<Generic<i32>>(),
            ServiceIdentifier::of::<Generic<String>>(),
            ServiceIdentifier::of::<Generic<String>>(),
        ],
    );
}

#[test]
fn no_circular_dependency_generic() {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(Generic::<String>(std::marker::PhantomData)));
    s.add(ServiceDescriptor::from_factory(
        Transient,
        service_factory!(Generic<i32>, |_s: required(Generic<String>)| Rc::new(
            Generic(std::marker::PhantomData)
        )),
    ));
    assert!(build(&s).get_service::<Generic<i32>>().unwrap().is_some());
}

#[test]
fn self_circular_dependency_with_interface() {
    let mut s = ServiceCollection::new();
    s.add(edge::<Value, dyn Service>()).add(
        descriptor(Transient, 1).with_dependencies(vec![Dependency::required::<dyn Service>()]),
    );
    cycle::<Value>(
        &build(&s),
        vec![
            ServiceIdentifier::of::<Value>(),
            ServiceIdentifier::of::<dyn Service>(),
            ServiceIdentifier::of::<dyn Service>(),
        ],
    );
}

#[test]
fn dependency_on_circular_dependency() {
    let mut s = ServiceCollection::new();
    s.add(edge::<usize, String>())
        .add(edge::<String, u8>())
        .add(edge::<u8, String>());
    cycle::<usize>(
        &build(&s),
        vec![
            ServiceIdentifier::of::<usize>(),
            ServiceIdentifier::of::<String>(),
            ServiceIdentifier::of::<u8>(),
            ServiceIdentifier::of::<String>(),
        ],
    );
}
