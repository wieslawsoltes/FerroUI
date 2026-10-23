//! Splat's `MicrosoftDependencyResolver.cs`: an `IDependencyResolver` backed by this container.
//! String contracts become service keys; `None` and `Some("")` remain distinct. Mutable
//! collections rebuild (and dispose the previous provider) after a registration change.
//! Supplying a built provider makes registrations immutable, matching upstream.

use crate::*;
use ferroui_splat::{
    IMutableDependencyResolver, IReadonlyDependencyResolver, LazyDisposal, RegistrationCallback,
    ServiceFactory, ServiceType as SplatType,
};
use ferroui_system::{threading::tasks::Task, IDisposable};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

struct NullService;
fn identifier(t: SplatType, contract: Option<&str>) -> ServiceIdentifier {
    ServiceIdentifier {
        service_type: ServiceType::from_parts(t.id(), t.name()),
        service_key: contract.map(ServiceKey::from),
    }
}
fn unwrap_null(value: ErasedService) -> Option<ErasedService> {
    if value.is::<NullService>() {
        None
    } else {
        Some(value)
    }
}

pub struct MicrosoftDependencyResolver {
    services: RefCell<ServiceCollection>,
    provider: RefCell<Option<ServiceProvider>>,
    immutable: Cell<bool>,
    disposed: Cell<bool>,
    constants: RefCell<Vec<Rc<dyn Fn()>>>,
    options: ServiceProviderOptions,
}
impl Default for MicrosoftDependencyResolver {
    fn default() -> Self {
        Self::new(ServiceCollection::new())
    }
}
impl MicrosoftDependencyResolver {
    pub fn new(services: ServiceCollection) -> Self {
        Self::with_options(services, ServiceProviderOptions::default())
    }
    pub fn with_options(services: ServiceCollection, options: ServiceProviderOptions) -> Self {
        Self {
            services: RefCell::new(services),
            provider: RefCell::new(None),
            immutable: Cell::new(false),
            disposed: Cell::new(false),
            constants: RefCell::new(Vec::new()),
            options,
        }
    }
    pub fn from_provider(provider: ServiceProvider) -> Self {
        let resolver = Self::default();
        *resolver.provider.borrow_mut() = Some(provider);
        resolver.immutable.set(true);
        resolver
    }
    fn ensure_active(&self) {
        assert!(
            !self.disposed.get(),
            "The dependency resolver has been disposed"
        );
    }
    fn invalidate(&self) {
        self.ensure_active();
        assert!(
            !self.immutable.get(),
            "This container has already been built and cannot be modified"
        );
        let previous = self.provider.borrow().clone();
        if let Some(provider) = previous {
            provider.dispose().unwrap_or_else(|e| panic!("{e}"));
        }
        self.provider.borrow_mut().take();
    }
    pub fn service_provider(&self) -> ServiceResult<ServiceProvider> {
        if self.disposed.get() {
            return Err(ServiceError::Disposed);
        }
        if let Some(provider) = self.provider.borrow().clone() {
            return Ok(provider);
        }
        let provider = self
            .services
            .borrow()
            .build_service_provider(self.options)?;
        *self.provider.borrow_mut() = Some(provider.clone());
        Ok(provider)
    }
    pub fn update_container(&self, services: ServiceCollection) {
        self.invalidate();
        *self.services.borrow_mut() = services;
    }
    pub fn update_provider(&self, provider: ServiceProvider) -> ServiceResult<()> {
        self.ensure_active();
        let previous = self.provider.borrow().clone();
        // Deviation (DEVIATIONS.md, Dependency injection): upstream's UpdateContainer disposes
        // the current provider even when it is the one passed in, which leaves the resolver with
        // a disposed provider; the port keeps it.
        if let Some(previous) = previous.filter(|previous| !previous.same_as(&provider)) {
            previous.dispose()?;
        }
        *self.provider.borrow_mut() = Some(provider);
        self.immutable.set(true);
        Ok(())
    }
    /// Installs a mutable adapter before Splat/ReactiveUI registers its defaults.
    pub fn use_splat(services: ServiceCollection) -> Rc<Self> {
        let resolver = Rc::new(Self::new(services));
        ferroui_splat::Locator::set_locator(resolver.clone());
        resolver
    }
    pub fn dispose(&self) -> ServiceResult<()> {
        if self.disposed.get() {
            return Ok(());
        }
        let provider = self.provider.borrow().clone();
        if let Some(provider) = provider {
            provider.dispose()?;
        }
        self.disposed.set(true);
        self.provider.borrow_mut().take();
        let constants = std::mem::take(&mut *self.constants.borrow_mut());
        for dispose in constants.into_iter().rev() {
            dispose();
        }
        Ok(())
    }
    pub fn dispose_async(&self) -> Task<()> {
        if self.disposed.replace(true) {
            return Task::from_result(());
        }
        let provider = self.provider.borrow_mut().take();
        let constants = std::mem::take(&mut *self.constants.borrow_mut());
        Task::spawn_local(async move {
            if let Some(provider) = provider {
                provider.dispose_async().await?;
            }
            for dispose in constants.into_iter().rev() {
                dispose();
            }
            Ok(())
        })
    }
    fn add(&self, descriptor: ServiceDescriptor) {
        self.invalidate();
        self.services.borrow_mut().add(descriptor);
    }
}
// Deviation (DEVIATIONS.md, Dependency injection): upstream throws from GetService and
// GetServices when the provider cannot resolve; the resolver trait returns an Option, so the
// error panics.
impl IReadonlyDependencyResolver for MicrosoftDependencyResolver {
    fn get_service_erased(
        &self,
        service_type: SplatType,
        contract: Option<&str>,
    ) -> Option<ErasedService> {
        self.service_provider()
            .and_then(|p| p.get_service_erased(&identifier(service_type, contract)))
            .unwrap_or_else(|e| panic!("{e}"))
            .and_then(unwrap_null)
    }
    fn get_services_erased(
        &self,
        service_type: SplatType,
        contract: Option<&str>,
    ) -> Vec<ErasedService> {
        self.service_provider()
            .and_then(|p| p.get_services_erased(&identifier(service_type, contract)))
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .filter_map(unwrap_null)
            .collect()
    }
}
impl IMutableDependencyResolver for MicrosoftDependencyResolver {
    fn has_registration_erased(&self, service_type: SplatType, contract: Option<&str>) -> bool {
        self.ensure_active();
        let id = identifier(service_type, contract);
        if self.immutable.get() {
            self.service_provider()
                .unwrap()
                .is_keyed_service(id.service_type, id.service_key.as_ref())
        } else {
            self.services
                .borrow()
                .descriptors()
                .iter()
                .any(|d| d.identifier == id)
        }
    }
    fn register_erased(
        &self,
        factory: ServiceFactory,
        service_type: SplatType,
        contract: Option<&str>,
    ) {
        self.add(ServiceDescriptor::from_erased(
            identifier(service_type, contract),
            ServiceLifetime::Transient,
            move |_, _| Ok(factory().unwrap_or_else(|| Rc::new(NullService))),
        ));
    }
    fn register_constant_erased(
        &self,
        value: Option<ErasedService>,
        disposal: Option<Rc<dyn Fn()>>,
        service_type: SplatType,
        contract: Option<&str>,
    ) {
        let value = value.unwrap_or_else(|| Rc::new(NullService));
        let mut d = ServiceDescriptor::from_erased(
            identifier(service_type, contract),
            ServiceLifetime::Singleton,
            move |_, _| Ok(value.clone()),
        );
        d.instance = true;
        self.add(d);
        if let Some(disposal) = disposal {
            self.constants.borrow_mut().push(disposal);
        }
    }
    fn register_lazy_singleton_erased(
        &self,
        factory: ServiceFactory,
        disposal: Option<LazyDisposal>,
        service_type: SplatType,
        contract: Option<&str>,
    ) {
        // As upstream's shared `Lazy<T>`: the value outlives the providers that a later
        // registration rebuilds, and is disposed with the resolver, once, if it was created.
        let value: Rc<OnceCell<ErasedService>> = Rc::new(OnceCell::new());
        let cell = value.clone();
        self.add(ServiceDescriptor::from_erased(
            identifier(service_type, contract),
            ServiceLifetime::Singleton,
            move |_, _| {
                Ok(cell
                    .get_or_init(|| factory().unwrap_or_else(|| Rc::new(NullService)))
                    .clone())
            },
        ));
        if let Some(dispose) = disposal {
            self.constants.borrow_mut().push(Rc::new(move || {
                if let Some(value) = value.get().filter(|v| !v.is::<NullService>()) {
                    dispose(value);
                }
            }));
        }
    }
    fn unregister_current_erased(&self, service_type: SplatType, contract: Option<&str>) {
        self.invalidate();
        let id = identifier(service_type, contract);
        let mut services = self.services.borrow_mut();
        if let Some(i) = services
            .descriptors()
            .iter()
            .rposition(|d| d.identifier == id)
        {
            services.remove_at(i);
        }
    }
    fn unregister_all_erased(&self, service_type: SplatType, contract: Option<&str>) {
        self.invalidate();
        let id = identifier(service_type, contract);
        let mut services = self.services.borrow_mut();
        for i in (0..services.len()).rev() {
            if services.descriptors()[i].identifier == id {
                services.remove_at(i);
            }
        }
    }
    fn service_registration_callback_erased(
        &self,
        _: SplatType,
        _: Option<&str>,
        _: RegistrationCallback,
    ) -> Rc<dyn IDisposable> {
        panic!("ServiceRegistrationCallback is not supported by MicrosoftDependencyResolver")
    }
}
impl IDisposable for MicrosoftDependencyResolver {
    fn dispose(&self) {
        self.dispose().unwrap_or_else(|e| panic!("{e}"));
    }
}
impl IAsyncDisposable for MicrosoftDependencyResolver {
    fn dispose_async(&self) -> Task<()> {
        self.dispose_async()
    }
}
