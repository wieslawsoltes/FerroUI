//! Disposal cases from DependencyInjectionSpecificationTests.cs and ServiceProviderContainerTests.cs.
use super::*;
use ferroui_system::{
    threading::tasks::{Task, TaskCompletionSource},
    IDisposable,
};
struct Resource {
    id: usize,
    log: Rc<RefCell<Vec<usize>>>,
}
impl IDisposable for Resource {
    fn dispose(&self) {
        self.log.borrow_mut().push(self.id);
    }
}
fn resource(
    lifetime: ServiceLifetime,
    id: usize,
    log: &Rc<RefCell<Vec<usize>>>,
) -> ServiceDescriptor {
    let log = log.clone();
    ServiceDescriptor::new::<Resource, _>(lifetime, move |_| {
        Ok(Rc::new(Resource {
            id,
            log: log.clone(),
        }))
    })
    .with_disposal::<Resource>()
}
#[test]
fn disposes_in_reverse_order_of_creation() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut s = ServiceCollection::new();
    s.add(resource(ServiceLifetime::Transient, 1, &log));
    s.add(
        ServiceDescriptor::new::<usize, _>(ServiceLifetime::Scoped, |p| {
            p.get_required_service::<Resource>()?;
            Ok(Rc::new(2))
        })
        .on_dispose::<usize>({
            let log = log.clone();
            move |n| log.borrow_mut().push(*n)
        }),
    );
    let p = s.build_service_provider(options()).unwrap();
    let scope = p.create_scope().unwrap();
    let value = scope
        .service_provider()
        .get_required_service::<usize>()
        .unwrap();
    scope
        .service_provider()
        .get_required_service::<Resource>()
        .unwrap();
    scope.dispose().unwrap();
    assert_eq!(&*log.borrow(), &[1, 2, 1]);
    assert_eq!(*value, 2);
    scope.dispose().unwrap();
    p.dispose().unwrap();
    assert_eq!(log.borrow().len(), 3);
}
#[test]
fn singleton_is_disposed_only_by_root_and_transient_by_owning_scope() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut s = ServiceCollection::new();
    s.add(resource(ServiceLifetime::Singleton, 1, &log));
    s.add(
        ServiceDescriptor::new::<usize, _>(ServiceLifetime::Transient, |_| Ok(Rc::new(2)))
            .on_dispose::<usize>({
                let log = log.clone();
                move |n| log.borrow_mut().push(*n)
            }),
    );
    let p = s.build_service_provider(options()).unwrap();
    let scope = p.create_scope().unwrap();
    scope
        .service_provider()
        .get_required_service::<Resource>()
        .unwrap();
    scope
        .service_provider()
        .get_required_service::<usize>()
        .unwrap();
    p.get_required_service::<usize>().unwrap();
    scope.dispose().unwrap();
    assert_eq!(&*log.borrow(), &[2]);
    p.dispose().unwrap();
    assert_eq!(&*log.borrow(), &[2, 2, 1]);
}
#[test]
fn instance_registrations_and_unresolved_services_are_not_disposed() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut s = ServiceCollection::new();
    s.add(resource(ServiceLifetime::Singleton, 1, &log));
    s.add_singleton_instance(Rc::new(Resource {
        id: 2,
        log: log.clone(),
    }));
    let p = s.build_service_provider(options()).unwrap();
    p.get_required_service::<Resource>().unwrap();
    p.dispose().unwrap();
    assert!(log.borrow().is_empty());
}
#[test]
fn root_disposal_invalidates_scopes_but_scopes_retain_disposal_ownership() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut s = ServiceCollection::new();
    s.add(resource(ServiceLifetime::Scoped, 1, &log));
    let p = s.build_service_provider(options()).unwrap();
    let scope = p.create_scope().unwrap();
    scope
        .service_provider()
        .get_required_service::<Resource>()
        .unwrap();
    p.dispose().unwrap();
    assert!(matches!(
        scope.service_provider().get_service::<Resource>(),
        Err(ServiceError::Disposed)
    ));
    assert!(matches!(p.create_scope(), Err(ServiceError::Disposed)));
    scope.dispose().unwrap();
    assert_eq!(&*log.borrow(), &[1]);
}
#[test]
fn disposal_callbacks_can_reenter_without_refcell_panics() {
    let holder = Rc::new(RefCell::new(None::<ServiceProvider>));
    let h = holder.clone();
    let mut s = ServiceCollection::new();
    s.add(
        ServiceDescriptor::new::<usize, _>(ServiceLifetime::Singleton, |_| Ok(Rc::new(1)))
            .on_dispose::<usize>(move |_| {
                let p = h.borrow().clone().unwrap();
                assert!(matches!(
                    p.get_service::<usize>(),
                    Err(ServiceError::Disposed)
                ));
                p.dispose().unwrap();
            }),
    );
    let p = s.build_service_provider(options()).unwrap();
    *holder.borrow_mut() = Some(p.clone());
    p.get_required_service::<usize>().unwrap();
    p.dispose().unwrap();
    holder.borrow_mut().take();
}
#[test]
fn async_disposal_waits_in_reverse_order_and_prefers_async() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let gate = TaskCompletionSource::new();
    let mut s = ServiceCollection::new();
    s.add(resource(ServiceLifetime::Singleton, 1, &log));
    let g = gate.clone();
    let l = log.clone();
    s.add(
        ServiceDescriptor::new::<usize, _>(ServiceLifetime::Singleton, |_| Ok(Rc::new(2)))
            .on_dispose::<usize>(|_| panic!("async should win"))
            .on_dispose_async::<usize>(move |_| {
                let task = g.task();
                let l = l.clone();
                Task::spawn_local(async move {
                    l.borrow_mut().push(20);
                    task.await?;
                    l.borrow_mut().push(2);
                    Ok(())
                })
            }),
    );
    let p = s.build_service_provider(options()).unwrap();
    p.get_required_service::<Resource>().unwrap();
    p.get_required_service::<usize>().unwrap();
    let done = p.dispose_async();
    assert!(!done.is_completed());
    assert_eq!(&*log.borrow(), &[20]);
    assert!(matches!(
        p.get_service::<usize>(),
        Err(ServiceError::Disposed)
    ));
    gate.set_result(());
    assert!(done.is_completed_successfully());
    assert_eq!(&*log.borrow(), &[20, 2, 1]);
}
#[test]
fn sync_disposal_of_async_only_service_can_be_retried_asynchronously() {
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let mut s = ServiceCollection::new();
    s.add(
        ServiceDescriptor::new::<usize, _>(ServiceLifetime::Singleton, |_| Ok(Rc::new(2)))
            .on_dispose_async::<usize>(move |_| {
                c.set(c.get() + 1);
                Task::from_result(())
            }),
    );
    let p = s.build_service_provider(options()).unwrap();
    p.get_required_service::<usize>().unwrap();
    assert_eq!(p.dispose(), Err(ServiceError::AsyncDisposalRequired));
    assert!(!p.is_disposed());
    assert!(p.dispose_async().is_completed_successfully());
    assert_eq!(count.get(), 1);
    p.dispose_async();
    assert_eq!(count.get(), 1);
}
#[test]
fn async_disposal_faults_task_and_stops_at_failure() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut s = ServiceCollection::new();
    s.add(resource(ServiceLifetime::Singleton, 1, &log));
    s.add(
        ServiceDescriptor::new::<usize, _>(ServiceLifetime::Singleton, |_| Ok(Rc::new(2)))
            .on_dispose_async::<usize>(|_| {
                Task::from_exception(Rc::new(ServiceError::Factory("dispose failed".into())))
            }),
    );
    let p = s.build_service_provider(options()).unwrap();
    p.get_required_service::<Resource>().unwrap();
    p.get_required_service::<usize>().unwrap();
    assert!(p.dispose_async().is_faulted());
    assert!(log.borrow().is_empty());
}
#[test]
fn service_created_by_a_factory_that_disposes_its_scope_is_disposed_immediately() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let l = log.clone();
    let mut s = ServiceCollection::new();
    s.add(
        ServiceDescriptor::new::<Resource, _>(ServiceLifetime::Transient, move |p| {
            p.dispose()?;
            Ok(Rc::new(Resource {
                id: 1,
                log: l.clone(),
            }))
        })
        .with_disposal::<Resource>(),
    );
    let p = s.build_service_provider(options()).unwrap();
    assert!(matches!(
        p.get_service::<Resource>(),
        Err(ServiceError::Disposed)
    ));
    assert_eq!(&*log.borrow(), &[1]);
}
#[test]
fn disposing_releases_cached_service_provider_cycles() {
    struct HoldsProvider {
        _provider: Rc<dyn IServiceProvider>,
    }
    let mut s = ServiceCollection::new();
    s.add_singleton::<HoldsProvider, _>(|p| {
        Ok(Rc::new(HoldsProvider {
            _provider: p.get_required_service()?,
        }))
    });
    let p = s.build_service_provider(options()).unwrap();
    let value = p.get_required_service::<HoldsProvider>().unwrap();
    let weak = Rc::downgrade(&value);
    drop(value);
    assert!(weak.upgrade().is_some());
    p.dispose().unwrap();
    assert!(weak.upgrade().is_none());
}
#[test]
fn scope_disposal_modes_prefer_the_requested_mode() {
    for asynchronous in [false, true] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let sync = log.clone();
        let other = log.clone();
        let mut s = ServiceCollection::new();
        s.add(
            ServiceDescriptor::new::<usize, _>(ServiceLifetime::Scoped, |_| Ok(Rc::new(1)))
                .on_dispose::<usize>(move |_| sync.borrow_mut().push("sync"))
                .on_dispose_async::<usize>(move |_| {
                    other.borrow_mut().push("async");
                    Task::from_result(())
                }),
        );
        let p = s.build_service_provider(options()).unwrap();
        let scope = p.create_async_scope().unwrap();
        scope
            .service_provider()
            .get_required_service::<usize>()
            .unwrap();
        if asynchronous {
            assert!(scope.dispose_async().is_completed_successfully());
        } else {
            scope.dispose().unwrap();
        }
        assert_eq!(
            &*log.borrow(),
            &[if asynchronous { "async" } else { "sync" }]
        );
    }
}
#[test]
fn failed_parent_factory_keeps_disposable_dependencies_for_scope_cleanup() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut s = ServiceCollection::new();
    s.add(resource(ServiceLifetime::Transient, 1, &log));
    s.add_transient::<usize, _>(|p| {
        p.get_required_service::<Resource>()?;
        Err(ServiceError::Factory("parent failed".into()))
    });
    let p = s.build_service_provider(options()).unwrap();
    assert!(p.get_service::<usize>().is_err());
    assert!(log.borrow().is_empty());
    p.dispose().unwrap();
    assert_eq!(&*log.borrow(), &[1]);
}
