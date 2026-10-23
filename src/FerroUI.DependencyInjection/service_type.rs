//! `ServiceLifetime.cs`, `IServiceProviderIsService.cs`, `IServiceProviderIsKeyedService.cs`,
//! `IKeyedServiceProvider.cs`, `KeyedService.cs` and the Rust service identifiers.

use std::any::{Any, TypeId};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ServiceLifetime {
    Singleton = 0,
    Scoped = 1,
    Transient = 2,
}

#[derive(Clone, Copy, Debug)]
pub struct ServiceType {
    id: TypeId,
    name: &'static str,
}
impl ServiceType {
    pub fn of<S: ?Sized + 'static>() -> Self {
        Self {
            id: TypeId::of::<S>(),
            name: std::any::type_name::<S>(),
        }
    }
    #[cfg(feature = "splat")]
    pub(crate) fn from_parts(id: TypeId, name: &'static str) -> Self {
        Self { id, name }
    }
    pub fn id(self) -> TypeId {
        self.id
    }
    pub fn name(self) -> &'static str {
        self.name
    }
}
impl PartialEq for ServiceType {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ServiceType {}
impl Hash for ServiceType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

trait KeyValue: fmt::Debug {
    fn as_any(&self) -> &dyn Any;
    fn equals(&self, other: &dyn KeyValue) -> bool;
    fn hash_value(&self) -> u64;
}
impl<T: Eq + Hash + fmt::Debug + 'static> KeyValue for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn equals(&self, other: &dyn KeyValue) -> bool {
        other.as_any().downcast_ref::<T>() == Some(self)
    }
    fn hash_value(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        TypeId::of::<T>().hash(&mut h);
        self.hash(&mut h);
        h.finish()
    }
}

/// An arbitrary value key, compared by type and value. `None` represents an unkeyed service.
#[derive(Clone)]
pub struct ServiceKey(Option<Rc<dyn KeyValue>>);
impl ServiceKey {
    pub fn new<T: Eq + Hash + fmt::Debug + 'static>(value: T) -> Self {
        Self(Some(Rc::new(value)))
    }
    /// Catch-all registration / query for all explicitly keyed registrations.
    pub const ANY: Self = Self(None);
    pub fn is_any(&self) -> bool {
        self.0.is_none()
    }
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.as_ref()?.as_any().downcast_ref()
    }
}
impl From<&str> for ServiceKey {
    fn from(s: &str) -> Self {
        Self::new(s.to_owned())
    }
}
impl From<String> for ServiceKey {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}
impl PartialEq for ServiceKey {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => a.equals(&**b),
            (None, None) => true,
            _ => false,
        }
    }
}
impl Eq for ServiceKey {}
impl Hash for ServiceKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.is_any().hash(state);
        if let Some(v) = &self.0 {
            v.hash_value().hash(state);
        }
    }
}
impl fmt::Debug for ServiceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(v) => fmt::Debug::fmt(v, f),
            None => f.write_str("AnyKey"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ServiceIdentifier {
    pub service_type: ServiceType,
    pub service_key: Option<ServiceKey>,
}
impl ServiceIdentifier {
    pub fn of<S: ?Sized + 'static>() -> Self {
        Self {
            service_type: ServiceType::of::<S>(),
            service_key: None,
        }
    }
    pub fn keyed<S: ?Sized + 'static>(key: Option<ServiceKey>) -> Self {
        Self {
            service_type: ServiceType::of::<S>(),
            service_key: key,
        }
    }
}

pub type ErasedService = Rc<dyn Any>;
pub fn erase_service<S: ?Sized + 'static>(value: Rc<S>) -> ErasedService {
    Rc::new(value)
}
pub fn unerase_service<S: ?Sized + 'static>(value: &ErasedService) -> Option<Rc<S>> {
    value.downcast_ref::<Rc<S>>().cloned()
}
pub type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceError {
    MissingService(ServiceIdentifier),
    CircularDependency(Vec<ServiceIdentifier>),
    ScopedFromRoot(ServiceIdentifier),
    ScopedInSingleton(ServiceIdentifier),
    Disposed,
    AsyncDisposalRequired,
    AnyKeySingleResolution,
    TypeMismatch(ServiceType),
    Factory(String),
    Validation(Vec<ServiceError>),
}
impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingService(id) => write!(f, "No service registered for {id:?}"),
            Self::CircularDependency(path) => write!(f, "Circular dependency: {path:?}"),
            Self::ScopedFromRoot(id) => write!(
                f,
                "Cannot resolve scoped service {id:?} from the root provider"
            ),
            Self::ScopedInSingleton(id) => {
                write!(f, "Cannot consume scoped service {id:?} from a singleton")
            }
            Self::Disposed => f.write_str("The service provider or scope has been disposed"),
            Self::AsyncDisposalRequired => {
                f.write_str("An async-only service requires dispose_async")
            }
            Self::AnyKeySingleResolution => {
                f.write_str("AnyKey may only be used to resolve multiple services")
            }
            Self::TypeMismatch(t) => {
                write!(f, "Factory returned a value of the wrong type for {t:?}")
            }
            Self::Factory(message) => f.write_str(message),
            Self::Validation(errors) => write!(f, "Service validation failed: {errors:?}"),
        }
    }
}
impl std::error::Error for ServiceError {}
