//! Behavioral ports of AsyncServiceScopeTests.cs (runtime v10.0.13).
use super::*;
struct FakeScope {
    provider: ServiceProvider,
    sync: Cell<usize>,
    asynchronous: Cell<usize>,
    supports_async: bool,
}
impl IDisposable for FakeScope {
    fn dispose(&self) {
        self.sync.set(self.sync.get() + 1);
    }
}
impl IAsyncDisposable for FakeScope {
    fn dispose_async(&self) -> Task<()> {
        self.asynchronous.set(self.asynchronous.get() + 1);
        Task::from_result(())
    }
}
impl IServiceScope for FakeScope {
    fn service_provider(&self) -> &ServiceProvider {
        &self.provider
    }
    fn async_disposable(&self) -> Option<&dyn IAsyncDisposable> {
        if self.supports_async {
            Some(self)
        } else {
            None
        }
    }
}
fn fake(supports_async: bool) -> Rc<FakeScope> {
    Rc::new(FakeScope {
        provider: build(&ServiceCollection::new()),
        sync: Cell::new(0),
        asynchronous: Cell::new(0),
        supports_async,
    })
}

#[test]
fn returns_service_provider_from_wrapped_scope() {
    let inner = fake(false);
    let scope = AsyncServiceScope::new(inner.clone());
    assert!(std::ptr::eq(&inner.provider, scope.service_provider()));
}

#[test]
fn calls_dispose_on_wrapped_sync_scope_on_dispose() {
    let inner = fake(false);
    let scope = AsyncServiceScope::new(inner.clone());
    scope.dispose().unwrap();
    assert_eq!(inner.sync.get(), 1);
    assert_eq!(inner.asynchronous.get(), 0);
}

#[test]
fn calls_dispose_on_wrapped_sync_scope_on_dispose_async() {
    let inner = fake(false);
    let scope = AsyncServiceScope::new(inner.clone());
    assert!(scope.dispose_async().is_completed_successfully());
    assert_eq!(inner.sync.get(), 1);
    assert_eq!(inner.asynchronous.get(), 0);
}

#[test]
fn calls_dispose_on_wrapped_async_scope_on_dispose() {
    let inner = fake(true);
    let scope = AsyncServiceScope::new(inner.clone());
    scope.dispose().unwrap();
    assert_eq!(inner.sync.get(), 1);
    assert_eq!(inner.asynchronous.get(), 0);
}

#[test]
fn calls_dispose_async_on_wrapped_sync_scope_on_dispose_async() {
    let inner = fake(true);
    let scope = AsyncServiceScope::new(inner.clone());
    assert!(scope.dispose_async().is_completed_successfully());
    assert_eq!(inner.sync.get(), 0);
    assert_eq!(inner.asynchronous.get(), 1);
}

#[test]
fn async_wrapper_returns_sync_scope_errors_as_faulted_tasks() {
    struct FailingScope(ServiceProvider);
    impl IDisposable for FailingScope {
        fn dispose(&self) {
            panic!("use fallible scope disposal");
        }
    }
    impl IServiceScope for FailingScope {
        fn service_provider(&self) -> &ServiceProvider {
            &self.0
        }
        fn dispose_scope(&self) -> ServiceResult<()> {
            Err(ServiceError::Factory("scope disposal failed".into()))
        }
    }
    let scope = AsyncServiceScope::new(Rc::new(FailingScope(build(&ServiceCollection::new()))));
    assert_eq!(
        scope.dispose(),
        Err(ServiceError::Factory("scope disposal failed".into()))
    );
    assert!(scope.dispose_async().is_faulted());
}
