//! `ServiceProvider.cs`, `ServiceProviderOptions.cs`, `IServiceScope.cs`, `IServiceScopeFactory.cs`,
//! `ISupportRequiredService.cs`, `AsyncServiceScope.cs`, `ServiceProviderServiceExtensions.cs`,
//! `ServiceProviderKeyedServiceExtensions.cs`, `ServiceLookup/CallSiteFactory.cs`,
//! `ServiceLookup/CallSiteChain.cs`, `ServiceLookup/CallSiteValidator.cs`,
//! `ServiceLookup/CallSiteRuntimeResolver.cs` and `ServiceLookup/ServiceProviderEngineScope.cs`.

use crate::*;
use ferroui_system::{threading::tasks::Task, IDisposable};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

#[derive(Clone, Copy, Debug, Default)]
pub struct ServiceProviderOptions {
    pub validate_scopes: bool,
    pub validate_on_build: bool,
}

pub trait IServiceProvider {
    fn ensure_active(&self) -> ServiceResult<()> {
        Ok(())
    }
    /// Explicit capability query replaces CLR interface casts for alternate providers.
    fn required_service_provider(&self) -> Option<&dyn ISupportRequiredService> {
        None
    }
    fn disposable(&self) -> Option<&dyn IDisposable> {
        None
    }
    fn required_service_erased(&self, id: &ServiceIdentifier) -> ServiceResult<ErasedService> {
        if let Some(provider) = self.required_service_provider() {
            provider.get_required_service_erased(id)
        } else {
            self.get_service_erased(id)?
                .ok_or_else(|| ServiceError::MissingService(id.clone()))
        }
    }
    fn get_service_erased(&self, id: &ServiceIdentifier) -> ServiceResult<Option<ErasedService>>;
    fn get_services_erased(&self, id: &ServiceIdentifier) -> ServiceResult<Vec<ErasedService>>;
}
/// Required-service resolution for alternate providers (`ISupportRequiredService.cs`).
pub trait ISupportRequiredService: IServiceProvider {
    fn get_required_service_erased(&self, id: &ServiceIdentifier) -> ServiceResult<ErasedService> {
        self.get_service_erased(id)?
            .ok_or_else(|| ServiceError::MissingService(id.clone()))
    }
}
impl ISupportRequiredService for ServiceProvider {}
pub trait IKeyedServiceProvider: IServiceProvider {
    fn get_keyed_service_erased(
        &self,
        service_type: ServiceType,
        key: Option<ServiceKey>,
    ) -> ServiceResult<Option<ErasedService>> {
        self.get_service_erased(&ServiceIdentifier {
            service_type,
            service_key: key,
        })
    }
    fn get_required_keyed_service_erased(
        &self,
        service_type: ServiceType,
        key: Option<ServiceKey>,
    ) -> ServiceResult<ErasedService> {
        let id = ServiceIdentifier {
            service_type,
            service_key: key,
        };
        self.get_service_erased(&id)?
            .ok_or(ServiceError::MissingService(id))
    }
}
pub trait IServiceProviderIsService {
    fn is_service(&self, service_type: ServiceType) -> bool;
}
pub trait IServiceProviderIsKeyedService {
    fn is_keyed_service(&self, service_type: ServiceType, key: Option<&ServiceKey>) -> bool;
}
pub trait IServiceScopeFactory {
    fn create_scope(&self) -> ServiceResult<ServiceScope>;
    fn create_async_scope(&self) -> ServiceResult<AsyncServiceScope> {
        Ok(AsyncServiceScope::new(Rc::new(self.create_scope()?)))
    }
}
pub trait IServiceScope: IDisposable {
    fn service_provider(&self) -> &ServiceProvider;
    fn async_disposable(&self) -> Option<&dyn IAsyncDisposable> {
        None
    }
    fn dispose_scope(&self) -> ServiceResult<()> {
        self.dispose();
        Ok(())
    }
}

pub trait ServiceProviderServiceExtensions: IServiceProvider {
    fn create_scope(&self) -> ServiceResult<ServiceScope> {
        self.get_required_service::<dyn IServiceScopeFactory>()?
            .create_scope()
    }
    fn create_async_scope(&self) -> ServiceResult<AsyncServiceScope> {
        Ok(AsyncServiceScope::new(Rc::new(self.create_scope()?)))
    }
    fn get_service<S: ?Sized + 'static>(&self) -> ServiceResult<Option<Rc<S>>> {
        self.get_keyed_service::<S>(None)
    }
    fn get_required_service<S: ?Sized + 'static>(&self) -> ServiceResult<Rc<S>> {
        let value = self.required_service_erased(&ServiceIdentifier::of::<S>())?;
        unerase_service(&value).ok_or(ServiceError::TypeMismatch(ServiceType::of::<S>()))
    }
    fn get_services<S: ?Sized + 'static>(&self) -> ServiceResult<Vec<Rc<S>>> {
        self.get_keyed_services::<S>(None)
    }
    fn get_keyed_service<S: ?Sized + 'static>(
        &self,
        key: Option<&ServiceKey>,
    ) -> ServiceResult<Option<Rc<S>>> {
        self.get_service_erased(&ServiceIdentifier::keyed::<S>(key.cloned()))?
            .map(|v| unerase_service(&v).ok_or(ServiceError::TypeMismatch(ServiceType::of::<S>())))
            .transpose()
    }
    fn get_required_keyed_service<S: ?Sized + 'static>(
        &self,
        key: Option<&ServiceKey>,
    ) -> ServiceResult<Rc<S>> {
        self.get_keyed_service::<S>(key)?.ok_or_else(|| {
            ServiceError::MissingService(ServiceIdentifier::keyed::<S>(key.cloned()))
        })
    }
    fn get_keyed_services<S: ?Sized + 'static>(
        &self,
        key: Option<&ServiceKey>,
    ) -> ServiceResult<Vec<Rc<S>>> {
        self.get_services_erased(&ServiceIdentifier::keyed::<S>(key.cloned()))?
            .iter()
            .map(|v| unerase_service(v).ok_or(ServiceError::TypeMismatch(ServiceType::of::<S>())))
            .collect()
    }
}
impl<T: IServiceProvider + ?Sized> ServiceProviderServiceExtensions for T {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    index: usize,
    key: Option<ServiceKey>,
}
struct Captured {
    value: ErasedService,
    dispose: Option<crate::service_descriptor::Dispose>,
    dispose_async: Option<crate::service_descriptor::DisposeAsync>,
}
#[derive(Default)]
struct ScopeState {
    disposed: Cell<bool>,
    provider: RefCell<Option<Weak<dyn IServiceProvider>>>,
    keyed_provider: RefCell<Option<Weak<dyn IKeyedServiceProvider>>>,
    scope_factory: RefCell<Option<Weak<dyn IServiceScopeFactory>>>,
    is_service: RefCell<Option<Weak<dyn IServiceProviderIsService>>>,
    is_keyed_service: RefCell<Option<Weak<dyn IServiceProviderIsKeyedService>>>,
    cache: RefCell<HashMap<CacheKey, ErasedService>>,
    disposables: RefCell<Vec<Captured>>,
}
/// How many validated resolutions a provider remembers before it starts again.
const MAX_VALIDATED: usize = 4096;
struct Root {
    descriptors: Vec<ServiceDescriptor>,
    lookup: HashMap<ServiceIdentifier, Vec<usize>>,
    options: ServiceProviderOptions,
    scope: Rc<ScopeState>,
    resolving: RefCell<Vec<(CacheKey, usize)>>,
    validated: RefCell<HashSet<(CacheKey, bool)>>,
}
/// Cloneable handle to a root provider or a scope. Neither Send nor Sync: services stay on the
/// creating thread. Cloning shares state; it never creates a new scope.
#[derive(Clone)]
pub struct ServiceProvider {
    root: Rc<Root>,
    scope: Rc<ScopeState>,
}
#[derive(Clone)]
pub struct ServiceScope {
    provider: ServiceProvider,
}
/// Wraps any synchronous scope, preferring its async-disposal capability when available.
#[derive(Clone)]
pub struct AsyncServiceScope {
    scope: Rc<dyn IServiceScope>,
}
impl AsyncServiceScope {
    pub fn new(scope: Rc<dyn IServiceScope>) -> Self {
        Self { scope }
    }
    pub fn service_provider(&self) -> &ServiceProvider {
        self.scope.service_provider()
    }
    pub fn dispose(&self) -> ServiceResult<()> {
        self.scope.dispose_scope()
    }
    pub fn dispose_async(&self) -> Task<()> {
        if let Some(scope) = self.scope.async_disposable() {
            scope.dispose_async()
        } else {
            match self.scope.dispose_scope() {
                Ok(()) => Task::from_result(()),
                Err(error) => Task::from_exception(Rc::new(error)),
            }
        }
    }
}
impl IDisposable for AsyncServiceScope {
    fn dispose(&self) {
        self.dispose().unwrap_or_else(|e| panic!("{e}"));
    }
}
impl IAsyncDisposable for AsyncServiceScope {
    fn dispose_async(&self) -> Task<()> {
        self.dispose_async()
    }
}
impl IServiceScope for AsyncServiceScope {
    fn service_provider(&self) -> &ServiceProvider {
        self.service_provider()
    }
    fn async_disposable(&self) -> Option<&dyn IAsyncDisposable> {
        Some(self)
    }
    fn dispose_scope(&self) -> ServiceResult<()> {
        self.dispose()
    }
}
impl IServiceScope for ServiceScope {
    fn async_disposable(&self) -> Option<&dyn IAsyncDisposable> {
        Some(self)
    }
    fn dispose_scope(&self) -> ServiceResult<()> {
        self.dispose()
    }
    fn service_provider(&self) -> &ServiceProvider {
        &self.provider
    }
}
impl ServiceScope {
    pub fn service_provider(&self) -> &ServiceProvider {
        &self.provider
    }
    pub fn dispose(&self) -> ServiceResult<()> {
        self.provider.dispose()
    }
    pub fn dispose_async(&self) -> Task<()> {
        self.provider.dispose_async()
    }
}
impl IDisposable for ServiceScope {
    fn dispose(&self) {
        self.dispose().unwrap_or_else(|e| panic!("{e}"));
    }
}
impl IAsyncDisposable for ServiceScope {
    fn dispose_async(&self) -> Task<()> {
        self.dispose_async()
    }
}

struct ResolutionGuard(Rc<Root>);
impl Drop for ResolutionGuard {
    fn drop(&mut self) {
        self.0.resolving.borrow_mut().pop();
    }
}

impl ServiceProvider {
    pub(crate) fn build(
        descriptors: Vec<ServiceDescriptor>,
        options: ServiceProviderOptions,
    ) -> ServiceResult<Self> {
        let scope = Rc::new(ScopeState::default());
        let mut lookup: HashMap<_, Vec<_>> = HashMap::new();
        for (i, descriptor) in descriptors.iter().enumerate() {
            lookup
                .entry(descriptor.identifier.clone())
                .or_default()
                .push(i);
        }
        let provider = Self {
            root: Rc::new(Root {
                descriptors,
                lookup,
                options,
                scope: scope.clone(),
                resolving: RefCell::new(Vec::new()),
                validated: RefCell::new(HashSet::new()),
            }),
            scope,
        };
        if options.validate_on_build {
            let mut done = HashSet::new();
            let errors: Vec<_> = provider
                .root
                .descriptors
                .iter()
                .enumerate()
                .filter_map(|(i, d)| {
                    provider
                        .validate(
                            i,
                            &d.identifier.service_key,
                            &mut Vec::new(),
                            &mut done,
                            false,
                            false,
                        )
                        .err()
                })
                .collect();
            if !errors.is_empty() {
                return Err(ServiceError::Validation(errors));
            }
        }
        Ok(provider)
    }
    pub(crate) fn ensure_active(&self) -> ServiceResult<()> {
        if self.scope.disposed.get() || self.root.scope.disposed.get() {
            Err(ServiceError::Disposed)
        } else {
            Ok(())
        }
    }
    /// Whether both handles are the same provider or scope.
    pub fn same_as(&self, other: &ServiceProvider) -> bool {
        Rc::ptr_eq(&self.root, &other.root) && Rc::ptr_eq(&self.scope, &other.scope)
    }
    pub fn is_disposed(&self) -> bool {
        self.ensure_active().is_err()
    }
    pub fn create_scope(&self) -> ServiceResult<ServiceScope> {
        self.ensure_active()?;
        Ok(ServiceScope {
            provider: Self {
                root: self.root.clone(),
                scope: Rc::new(ScopeState::default()),
            },
        })
    }
    pub fn create_async_scope(&self) -> ServiceResult<AsyncServiceScope> {
        Ok(AsyncServiceScope::new(Rc::new(self.create_scope()?)))
    }
    pub fn get_service<S: ?Sized + 'static>(&self) -> ServiceResult<Option<Rc<S>>> {
        ServiceProviderServiceExtensions::get_service(self)
    }
    pub fn get_required_service<S: ?Sized + 'static>(&self) -> ServiceResult<Rc<S>> {
        ServiceProviderServiceExtensions::get_required_service(self)
    }
    pub fn get_services<S: ?Sized + 'static>(&self) -> ServiceResult<Vec<Rc<S>>> {
        ServiceProviderServiceExtensions::get_services(self)
    }
    pub fn get_keyed_service<S: ?Sized + 'static>(
        &self,
        key: Option<&ServiceKey>,
    ) -> ServiceResult<Option<Rc<S>>> {
        ServiceProviderServiceExtensions::get_keyed_service(self, key)
    }
    pub fn get_required_keyed_service<S: ?Sized + 'static>(
        &self,
        key: Option<&ServiceKey>,
    ) -> ServiceResult<Rc<S>> {
        ServiceProviderServiceExtensions::get_required_keyed_service(self, key)
    }
    pub fn get_keyed_services<S: ?Sized + 'static>(
        &self,
        key: Option<&ServiceKey>,
    ) -> ServiceResult<Vec<Rc<S>>> {
        ServiceProviderServiceExtensions::get_keyed_services(self, key)
    }
    fn is_builtin(service_type: ServiceType) -> bool {
        [
            ServiceType::of::<dyn IServiceProvider>(),
            ServiceType::of::<dyn IKeyedServiceProvider>(),
            ServiceType::of::<dyn IServiceScopeFactory>(),
            ServiceType::of::<dyn IServiceProviderIsService>(),
            ServiceType::of::<dyn IServiceProviderIsKeyedService>(),
        ]
        .contains(&service_type)
    }
    fn builtin(&self, id: &ServiceIdentifier) -> Option<ErasedService> {
        if id.service_key.is_some() {
            return None;
        }
        let t = id.service_type;
        fn cached<T: ?Sized + 'static>(
            slot: &RefCell<Option<Weak<T>>>,
            create: impl FnOnce() -> Rc<T>,
        ) -> ErasedService {
            if let Some(value) = slot.borrow().as_ref().and_then(Weak::upgrade) {
                return erase_service(value);
            }
            let value = create();
            *slot.borrow_mut() = Some(Rc::downgrade(&value));
            erase_service(value)
        }
        if t == ServiceType::of::<dyn IServiceProvider>() {
            return Some(cached(&self.scope.provider, || Rc::new(self.clone())));
        }
        if t == ServiceType::of::<dyn IKeyedServiceProvider>() {
            return Some(cached(&self.scope.keyed_provider, || Rc::new(self.clone())));
        }
        let root = Self {
            root: self.root.clone(),
            scope: self.root.scope.clone(),
        };
        if t == ServiceType::of::<dyn IServiceScopeFactory>() {
            return Some(cached(&self.root.scope.scope_factory, || Rc::new(root)));
        }
        if t == ServiceType::of::<dyn IServiceProviderIsService>() {
            return Some(cached(&self.root.scope.is_service, || Rc::new(root)));
        }
        if t == ServiceType::of::<dyn IServiceProviderIsKeyedService>() {
            return Some(cached(&self.root.scope.is_keyed_service, || Rc::new(root)));
        }
        None
    }
    fn matching(&self, id: &ServiceIdentifier, all: bool) -> ServiceResult<Vec<CacheKey>> {
        if id.service_key.as_ref().is_some_and(ServiceKey::is_any) {
            if !all {
                return Err(ServiceError::AnyKeySingleResolution);
            }
            return Ok(self
                .root
                .descriptors
                .iter()
                .enumerate()
                .filter(|(_, d)| {
                    d.service_type() == id.service_type
                        && d.service_key().is_some_and(|k| !k.is_any())
                })
                .map(|(index, d)| CacheKey {
                    index,
                    key: d.identifier.service_key.clone(),
                })
                .collect());
        }
        if let Some(indices) = self.root.lookup.get(id) {
            return Ok(if all {
                indices
                    .iter()
                    .map(|&index| CacheKey {
                        index,
                        key: id.service_key.clone(),
                    })
                    .collect()
            } else {
                vec![CacheKey {
                    index: *indices.last().unwrap(),
                    key: id.service_key.clone(),
                }]
            });
        }
        // AnyKey is a fallback for a single lookup, never for enumeration.
        if !all && id.service_key.is_some() {
            let any = ServiceIdentifier {
                service_type: id.service_type,
                service_key: Some(ServiceKey::ANY),
            };
            if let Some(indices) = self.root.lookup.get(&any) {
                return Ok(vec![CacheKey {
                    index: *indices.last().unwrap(),
                    key: id.service_key.clone(),
                }]);
            }
        }
        Ok(Vec::new())
    }
    /// Checks the dependencies below a registration: scoped services reached from a singleton or
    /// the root, missing services and cycles. `done` holds the subtrees already found valid in
    /// this pass, with the two flags they were checked under, so that a graph whose branches meet
    /// again (diamonds) is checked in linear time rather than once per path.
    fn validate(
        &self,
        index: usize,
        key: &Option<ServiceKey>,
        path: &mut Vec<CacheKey>,
        done: &mut HashSet<(CacheKey, bool, bool)>,
        singleton: bool,
        root_resolution: bool,
    ) -> ServiceResult<()> {
        let d = &self.root.descriptors[index];
        let id = ServiceIdentifier {
            service_type: d.service_type(),
            service_key: key.clone(),
        };
        if self.root.options.validate_scopes && d.lifetime == ServiceLifetime::Scoped {
            if singleton {
                return Err(ServiceError::ScopedInSingleton(id));
            }
            if root_resolution {
                return Err(ServiceError::ScopedFromRoot(id));
            }
        }
        let cache_key = CacheKey {
            index,
            key: key.clone(),
        };
        // A subtree found valid has no cycle, so it cannot lead back to the current path either.
        if done.contains(&(cache_key.clone(), singleton, root_resolution)) {
            return Ok(());
        }
        if path.contains(&cache_key) {
            let mut ids: Vec<_> = path
                .iter()
                .map(|k| ServiceIdentifier {
                    service_type: self.root.descriptors[k.index].service_type(),
                    service_key: k.key.clone(),
                })
                .collect();
            ids.push(id);
            return Err(ServiceError::CircularDependency(ids));
        }
        let memo = (cache_key.clone(), singleton, root_resolution);
        path.push(cache_key);
        let singleton = singleton || d.lifetime == ServiceLifetime::Singleton;
        for dependency in &d.dependencies {
            let id = dependency.identifier_for(key);
            if id.service_key.is_none() && Self::is_builtin(id.service_type) {
                continue;
            }
            let matches = if dependency.kind == DependencyKind::Enumerable
                && self.enumerable_override(&id).is_some()
            {
                vec![self.enumerable_override(&id).expect("enumerable override")]
            } else if dependency.lookup_mode == ServiceKeyLookupMode::InheritKey
                && key.as_ref().is_some_and(ServiceKey::is_any)
                && dependency.kind != DependencyKind::Enumerable
            {
                self.root
                    .lookup
                    .get(&id)
                    .and_then(|indices| indices.last())
                    .map(|&index| {
                        vec![CacheKey {
                            index,
                            key: key.clone(),
                        }]
                    })
                    .unwrap_or_default()
            } else {
                self.matching(&id, dependency.kind == DependencyKind::Enumerable)?
            };
            if matches.is_empty() && dependency.kind == DependencyKind::Required {
                return Err(ServiceError::MissingService(id));
            }
            for child in matches {
                self.validate(
                    child.index,
                    &child.key,
                    path,
                    done,
                    singleton,
                    root_resolution,
                )?;
            }
        }
        path.pop();
        done.insert(memo);
        Ok(())
    }
    fn enumerable_override(&self, id: &ServiceIdentifier) -> Option<CacheKey> {
        self.root
            .descriptors
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, d)| {
                (d.enumerable
                    .as_ref()
                    .is_some_and(|(element, _)| *element == id.service_type)
                    && d.service_key() == id.service_key.as_ref())
                .then(|| CacheKey {
                    index,
                    key: id.service_key.clone(),
                })
            })
    }
    fn resolve(&self, key: CacheKey) -> ServiceResult<ErasedService> {
        self.ensure_active()?;
        let d = &self.root.descriptors[key.index];
        let provider = if d.lifetime == ServiceLifetime::Singleton {
            Self {
                root: self.root.clone(),
                scope: self.root.scope.clone(),
            }
        } else {
            self.clone()
        };
        let validation_key = (key.clone(), Rc::ptr_eq(&self.scope, &self.root.scope));
        if !self.root.validated.borrow().contains(&validation_key) {
            self.validate(
                key.index,
                &key.key,
                &mut Vec::new(),
                &mut HashSet::new(),
                false,
                validation_key.1,
            )?;
            // The set is a cache: keys of catch-all registrations come from callers and are not
            // bounded, so it starts again past a size.
            let mut validated = self.root.validated.borrow_mut();
            if validated.len() >= MAX_VALIDATED {
                validated.clear();
            }
            validated.insert(validation_key);
        }
        if d.lifetime != ServiceLifetime::Transient {
            if let Some(value) = provider.scope.cache.borrow().get(&key) {
                return Ok(value.clone());
            }
        }
        let context = (key.clone(), Rc::as_ptr(&provider.scope) as usize);
        if self.root.resolving.borrow().contains(&context) {
            let mut ids: Vec<_> = self
                .root
                .resolving
                .borrow()
                .iter()
                .map(|(k, _)| ServiceIdentifier {
                    service_type: self.root.descriptors[k.index].service_type(),
                    service_key: k.key.clone(),
                })
                .collect();
            ids.push(ServiceIdentifier {
                service_type: d.service_type(),
                service_key: key.key.clone(),
            });
            return Err(ServiceError::CircularDependency(ids));
        }
        self.root.resolving.borrow_mut().push(context);
        let _guard = ResolutionGuard(self.root.clone());
        let value = (d.factory)(&provider, key.key.as_ref())?;
        if d.dispose.is_some() || d.dispose_async.is_some() {
            let captured = Captured {
                value: value.clone(),
                dispose: d.dispose.clone(),
                dispose_async: d.dispose_async.clone(),
            };
            if provider.is_disposed() {
                // A user factory can dispose its scope while constructing. Never publish that value.
                if let Some(dispose) = &captured.dispose {
                    dispose(&captured.value);
                } else if let Some(dispose) = &captured.dispose_async {
                    // Tasks are already running on the local executor; dropping the handle does not cancel them.
                    drop(dispose(&captured.value));
                }
                return Err(ServiceError::Disposed);
            }
            provider.scope.disposables.borrow_mut().push(captured);
        }
        provider.ensure_active()?;
        if d.lifetime != ServiceLifetime::Transient {
            provider.scope.cache.borrow_mut().insert(key, value.clone());
        }
        Ok(value)
    }
    /// Disposes captured services in reverse construction order. Async-only services require
    /// `dispose_async`; that error leaves the scope active so the caller can retry asynchronously.
    pub fn dispose(&self) -> ServiceResult<()> {
        if self.scope.disposed.get() {
            return Ok(());
        }
        if self
            .scope
            .disposables
            .borrow()
            .iter()
            .any(|d| d.dispose.is_none() && d.dispose_async.is_some())
        {
            return Err(ServiceError::AsyncDisposalRequired);
        }
        self.scope.disposed.set(true);
        let disposables = std::mem::take(&mut *self.scope.disposables.borrow_mut());
        let cache = std::mem::take(&mut *self.scope.cache.borrow_mut());
        for captured in disposables.into_iter().rev() {
            if let Some(dispose) = captured.dispose {
                dispose(&captured.value);
            }
        }
        drop(cache);
        Ok(())
    }
    pub fn dispose_async(&self) -> Task<()> {
        if self.scope.disposed.replace(true) {
            return Task::from_result(());
        }
        let disposables = std::mem::take(&mut *self.scope.disposables.borrow_mut());
        let cache = std::mem::take(&mut *self.scope.cache.borrow_mut());
        Task::spawn_local(async move {
            for captured in disposables.into_iter().rev() {
                if let Some(dispose) = captured.dispose_async {
                    dispose(&captured.value).await?;
                } else if let Some(dispose) = captured.dispose {
                    dispose(&captured.value);
                }
            }
            drop(cache);
            Ok(())
        })
    }
}
impl IServiceProvider for ServiceProvider {
    fn ensure_active(&self) -> ServiceResult<()> {
        self.ensure_active()
    }
    fn required_service_provider(&self) -> Option<&dyn ISupportRequiredService> {
        Some(self)
    }
    fn disposable(&self) -> Option<&dyn IDisposable> {
        Some(self)
    }
    fn get_service_erased(&self, id: &ServiceIdentifier) -> ServiceResult<Option<ErasedService>> {
        self.ensure_active()?;
        if let Some(value) = self.builtin(id) {
            return Ok(Some(value));
        }
        self.matching(id, false)?
            .into_iter()
            .next()
            .map(|key| self.resolve(key))
            .transpose()
    }
    fn get_services_erased(&self, id: &ServiceIdentifier) -> ServiceResult<Vec<ErasedService>> {
        self.ensure_active()?;
        if let Some(key) = self.enumerable_override(id) {
            let enumerate = self.root.descriptors[key.index]
                .enumerable
                .as_ref()
                .expect("enumerable")
                .1;
            return Ok(enumerate(&self.resolve(key)?));
        }
        self.matching(id, true)?
            .into_iter()
            .map(|key| self.resolve(key))
            .collect()
    }
}
impl IKeyedServiceProvider for ServiceProvider {}
impl IServiceProviderIsService for ServiceProvider {
    fn is_service(&self, service_type: ServiceType) -> bool {
        self.is_keyed_service(service_type, None)
    }
}
impl IServiceProviderIsKeyedService for ServiceProvider {
    fn is_keyed_service(&self, service_type: ServiceType, key: Option<&ServiceKey>) -> bool {
        (key.is_none() && Self::is_builtin(service_type))
            || self.root.lookup.contains_key(&ServiceIdentifier {
                service_type,
                service_key: key.cloned(),
            })
            || (key.is_some()
                && self.root.lookup.contains_key(&ServiceIdentifier {
                    service_type,
                    service_key: Some(ServiceKey::ANY),
                }))
    }
}
impl IServiceScopeFactory for ServiceProvider {
    fn create_scope(&self) -> ServiceResult<ServiceScope> {
        self.create_scope()
    }
}
// Deviation (DEVIATIONS.md, Dependency injection): upstream's Dispose throws when a service has
// only asynchronous disposal; the trait method has no result, so it panics with that error.
impl IDisposable for ServiceProvider {
    fn dispose(&self) {
        self.dispose().unwrap_or_else(|e| panic!("{e}"));
    }
}
impl IAsyncDisposable for ServiceProvider {
    fn dispose_async(&self) -> Task<()> {
        self.dispose_async()
    }
}
