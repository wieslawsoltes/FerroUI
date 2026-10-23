//! `ServiceDescriptor.cs`, `ServiceKeyLookupMode.cs`, and constructor call-site metadata.

use crate::*;
use ferroui_system::{threading::tasks::Task, IDisposable};
use std::rc::Rc;

/// Async disposal uses FerroUI's local task executor; no external runtime is required.
pub trait IAsyncDisposable {
    fn dispose_async(&self) -> Task<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceKeyLookupMode {
    NullKey,
    ExplicitKey,
    InheritKey,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyKind {
    Required,
    Optional,
    Enumerable,
}

/// A constructor parameter's call site, used without running user code during validation.
#[derive(Clone, Debug)]
pub struct Dependency {
    pub identifier: ServiceIdentifier,
    pub kind: DependencyKind,
    pub lookup_mode: ServiceKeyLookupMode,
}
impl Dependency {
    pub fn required<S: ?Sized + 'static>() -> Self {
        Self {
            identifier: ServiceIdentifier::of::<S>(),
            kind: DependencyKind::Required,
            lookup_mode: ServiceKeyLookupMode::NullKey,
        }
    }
    pub fn keyed<S: ?Sized + 'static>(key: Option<ServiceKey>) -> Self {
        Self {
            lookup_mode: if key.is_some() {
                ServiceKeyLookupMode::ExplicitKey
            } else {
                ServiceKeyLookupMode::NullKey
            },
            identifier: ServiceIdentifier::keyed::<S>(key),
            kind: DependencyKind::Required,
        }
    }
    pub fn inherited<S: ?Sized + 'static>() -> Self {
        Self {
            lookup_mode: ServiceKeyLookupMode::InheritKey,
            ..Self::required::<S>()
        }
    }
    pub fn optional(mut self) -> Self {
        self.kind = DependencyKind::Optional;
        self
    }
    pub fn enumerable(mut self) -> Self {
        self.kind = DependencyKind::Enumerable;
        self
    }
    pub(crate) fn identifier_for(&self, key: &Option<ServiceKey>) -> ServiceIdentifier {
        let mut id = self.identifier.clone();
        if self.lookup_mode == ServiceKeyLookupMode::InheritKey {
            id.service_key = key.clone();
        }
        id
    }
}

pub type ErasedFactory =
    Rc<dyn Fn(&ServiceProvider, Option<&ServiceKey>) -> ServiceResult<ErasedService>>;
pub(crate) type Enumerate = fn(&ErasedService) -> Vec<ErasedService>;
pub(crate) type Dispose = Rc<dyn Fn(&ErasedService)>;
pub(crate) type DisposeAsync = Rc<dyn Fn(&ErasedService) -> Task<()>>;

/// A registration. Construct typed factories with `new` or `from_factory`; erased registration
/// exists for adapters. Instance registrations are externally owned and never disposed.
#[derive(Clone)]
pub struct ServiceDescriptor {
    pub(crate) identity: Rc<()>,
    pub(crate) enumerable: Option<(ServiceType, Enumerate)>,
    instance_value: Option<ErasedService>,
    pub(crate) identifier: ServiceIdentifier,
    pub(crate) lifetime: ServiceLifetime,
    pub(crate) factory: ErasedFactory,
    pub(crate) dependencies: Vec<Dependency>,
    pub(crate) implementation_type: Option<ServiceType>,
    pub(crate) dispose: Option<Dispose>,
    pub(crate) dispose_async: Option<DisposeAsync>,
    pub(crate) instance: bool,
}
impl ServiceDescriptor {
    pub fn new<S: ?Sized + 'static, F>(lifetime: ServiceLifetime, factory: F) -> Self
    where
        F: Fn(&ServiceProvider) -> ServiceResult<Rc<S>> + 'static,
    {
        Self::keyed::<S, _>(lifetime, None, move |p, _| factory(p))
    }
    pub fn keyed<S: ?Sized + 'static, F>(
        lifetime: ServiceLifetime,
        key: Option<ServiceKey>,
        factory: F,
    ) -> Self
    where
        F: Fn(&ServiceProvider, Option<&ServiceKey>) -> ServiceResult<Rc<S>> + 'static,
    {
        Self::from_erased(ServiceIdentifier::keyed::<S>(key), lifetime, move |p, k| {
            factory(p, k).map(erase_service)
        })
    }
    pub fn from_erased(
        identifier: ServiceIdentifier,
        lifetime: ServiceLifetime,
        factory: impl Fn(&ServiceProvider, Option<&ServiceKey>) -> ServiceResult<ErasedService>
            + 'static,
    ) -> Self {
        Self {
            identity: Rc::new(()),
            enumerable: None,
            instance_value: None,
            identifier,
            lifetime,
            factory: Rc::new(factory),
            dependencies: Vec::new(),
            implementation_type: None,
            dispose: None,
            dispose_async: None,
            instance: false,
        }
    }
    pub fn instance<S: ?Sized + 'static>(value: Rc<S>) -> Self {
        let instance = erase_service(value.clone());
        let mut descriptor = Self::new(ServiceLifetime::Singleton, move |_| Ok(value.clone()));
        descriptor.instance_value = Some(instance);
        descriptor.instance = true;
        descriptor
    }
    /// Explicit enumerable registration overrides the collection synthesized from individual services.
    pub fn enumerable_instance<S: ?Sized + 'static>(values: Vec<Rc<S>>) -> Self {
        let mut descriptor = Self::instance(Rc::new(values));
        descriptor.enumerable = Some((ServiceType::of::<S>(), |value| {
            unerase_service::<Vec<Rc<S>>>(value)
                .expect("typed enumerable")
                .iter()
                .cloned()
                .map(erase_service)
                .collect()
        }));
        descriptor
    }
    pub fn from_factory<S: ?Sized + 'static>(
        lifetime: ServiceLifetime,
        factory: ObjectFactory<S>,
    ) -> Self {
        let mut descriptor = Self::keyed(lifetime, None, move |p, k| (factory.factory)(p, k));
        descriptor.dependencies = factory.dependencies;
        descriptor
    }
    pub fn with_key(mut self, key: Option<ServiceKey>) -> Self {
        self.identifier.service_key = key;
        self
    }
    pub fn with_dependencies(mut self, dependencies: Vec<Dependency>) -> Self {
        self.dependencies = dependencies;
        self
    }
    /// Identity used by TryAddEnumerable to distinguish implementations of the same service.
    pub fn with_implementation<I: ?Sized + 'static>(mut self) -> Self {
        self.implementation_type = Some(ServiceType::of::<I>());
        self
    }
    pub fn with_disposal<S: ?Sized + IDisposable + 'static>(self) -> Self {
        self.on_dispose::<S>(|value| value.dispose())
    }
    pub fn with_async_disposal<S: ?Sized + IAsyncDisposable + 'static>(self) -> Self {
        self.on_dispose_async::<S>(|value| value.dispose_async())
    }
    pub fn on_dispose<S: ?Sized + 'static>(mut self, dispose: impl Fn(&S) + 'static) -> Self {
        assert_eq!(
            self.service_type(),
            ServiceType::of::<S>(),
            "disposer must match service type"
        );
        assert!(
            !self.instance,
            "externally supplied instances are not owned by the provider"
        );
        self.dispose = Some(Rc::new(move |v| {
            dispose(&*unerase_service::<S>(v).expect("typed factory"))
        }));
        self
    }
    pub fn on_dispose_async<S: ?Sized + 'static>(
        mut self,
        dispose: impl Fn(Rc<S>) -> Task<()> + 'static,
    ) -> Self {
        assert_eq!(
            self.service_type(),
            ServiceType::of::<S>(),
            "disposer must match service type"
        );
        assert!(
            !self.instance,
            "externally supplied instances are not owned by the provider"
        );
        self.dispose_async = Some(Rc::new(move |v| {
            dispose(unerase_service::<S>(v).expect("typed factory"))
        }));
        self
    }
    pub fn implementation_type(&self) -> Option<ServiceType> {
        self.implementation_type
    }
    pub fn implementation_instance(&self) -> Option<ErasedService> {
        self.instance_value.clone()
    }
    pub fn implementation_factory(&self) -> Option<crate::service_descriptor::ErasedFactory> {
        if self.instance {
            None
        } else {
            Some(self.factory.clone())
        }
    }
    pub fn service_type(&self) -> ServiceType {
        self.identifier.service_type
    }
    pub fn service_key(&self) -> Option<&ServiceKey> {
        self.identifier.service_key.as_ref()
    }
    pub fn lifetime(&self) -> ServiceLifetime {
        self.lifetime
    }
    pub fn is_keyed_service(&self) -> bool {
        self.service_key().is_some()
    }
    pub fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }
}

impl std::fmt::Debug for ServiceDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceDescriptor")
            .field("identifier", &self.identifier)
            .field("lifetime", &self.lifetime)
            .field("implementation_type", &self.implementation_type)
            .field("instance", &self.instance)
            .finish()
    }
}
impl std::fmt::Display for ServiceDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
