//! `ActivatorUtilities.cs`, `ObjectFactory.cs`, `ObjectFactoryT.cs`,
//! `FromKeyedServicesAttribute.cs` and `ServiceKeyAttribute.cs`.
//! Reflection is replaced by typed constructor factories and explicit dependency metadata.

use crate::*;
use std::rc::Rc;

type Constructor<S> =
    Rc<dyn Fn(&dyn IServiceProvider, Option<&ServiceKey>) -> ServiceResult<Rc<S>>>;
pub struct ObjectFactory<S: ?Sized> {
    pub(crate) factory: Constructor<S>,
    pub(crate) dependencies: Vec<Dependency>,
}
impl<S: ?Sized> Clone for ObjectFactory<S> {
    fn clone(&self) -> Self {
        Self {
            factory: self.factory.clone(),
            dependencies: self.dependencies.clone(),
        }
    }
}
impl<S: ?Sized + 'static> ObjectFactory<S> {
    pub fn new(
        dependencies: Vec<Dependency>,
        factory: impl Fn(&dyn IServiceProvider, Option<&ServiceKey>) -> ServiceResult<Rc<S>> + 'static,
    ) -> Self {
        Self {
            dependencies,
            factory: Rc::new(factory),
        }
    }
}

/// Creates unregistered instances from the same factories as registrations. Such instances
/// belong to the caller, not to a provider's disposal list. Explicit constructor arguments can
/// be captured by the factory; Rust checks their types and the selected constructor.
pub struct ActivatorUtilities;
impl ActivatorUtilities {
    pub fn create_factory<S: ?Sized + 'static>(factory: ObjectFactory<S>) -> ObjectFactory<S> {
        factory
    }
    pub fn create_instance<S: ?Sized + 'static>(
        provider: &dyn IServiceProvider,
        factory: &ObjectFactory<S>,
    ) -> ServiceResult<Rc<S>> {
        Self::create_keyed_instance(provider, None, factory)
    }
    pub fn create_keyed_instance<S: ?Sized + 'static>(
        provider: &dyn IServiceProvider,
        key: Option<&ServiceKey>,
        factory: &ObjectFactory<S>,
    ) -> ServiceResult<Rc<S>> {
        provider.ensure_active()?;
        (factory.factory)(provider, key)
    }
    pub fn get_service_or_create_instance<S: ?Sized + 'static>(
        provider: &dyn IServiceProvider,
        factory: &ObjectFactory<S>,
    ) -> ServiceResult<Rc<S>> {
        match provider.get_service::<S>()? {
            Some(value) => Ok(value),
            None => Self::create_instance(provider, factory),
        }
    }
}

/// Writes a factory and its validation metadata from typed constructor parameters.
///
/// Parameters use `required(T)`, `optional(T)`, `all(T)`, `keyed(T, key)`,
/// `inherited(T)`, `key()` (the current `Option<ServiceKey>`), or `typed_key(T)`
/// (a cloned key of type `T`, checked at activation).
/// The body returns `Rc<S>`; use `ObjectFactory::new` for a fallible constructor.
/// ```
/// # use ferroui_dependency_injection::*;
/// # use std::rc::Rc;
/// let factory = service_factory!(String, |name: required(str)| Rc::new(name.to_string()));
/// let descriptor = ServiceDescriptor::from_factory(ServiceLifetime::Transient, factory);
/// ```
#[macro_export]
macro_rules! service_factory {
    ($service:ty, || $body:expr) => { $crate::ObjectFactory::<$service>::new(Vec::new(), move |_, _| Ok($body)) };
    ($service:ty, |$($name:ident : $kind:ident($($args:tt)*)),* $(,)?| $body:expr) => {{
        let dependencies: Vec<Option<$crate::Dependency>> = vec![$($crate::service_factory!(@dependency $kind($($args)*))),*];
        $crate::ObjectFactory::<$service>::new(dependencies.iter().flatten().cloned().collect(), move |provider, service_key| {
            let mut parameters = dependencies.iter();
            $(let dependency = parameters.next().expect("constructor parameter");
              let $name = $crate::service_factory!(@resolve provider, service_key, dependency, $kind($($args)*));)*
            let _ = &mut parameters;
            let _ = (&provider, &service_key);
            Ok($body)
        })
    }};
    (@dependency required($t:ty)) => { Some($crate::Dependency::required::<$t>()) };
    (@dependency optional($t:ty)) => { Some($crate::Dependency::required::<$t>().optional()) };
    (@dependency all($t:ty)) => { Some($crate::Dependency::required::<$t>().enumerable()) };
    (@dependency keyed($t:ty, $key:expr)) => { Some($crate::Dependency::keyed::<$t>(Some(($key).into()))) };
    (@dependency inherited($t:ty)) => { Some($crate::Dependency::inherited::<$t>()) };
    (@dependency typed_key($t:ty)) => { None::<$crate::Dependency> };
    (@dependency key()) => { None::<$crate::Dependency> };
    (@resolve $p:ident, $k:ident, $d:ident, required($t:ty)) => {{ let _ = $d; $crate::ServiceProviderServiceExtensions::get_required_service::<$t>($p)? }};
    (@resolve $p:ident, $k:ident, $d:ident, optional($t:ty)) => {{ let _ = $d; $crate::ServiceProviderServiceExtensions::get_service::<$t>($p)? }};
    (@resolve $p:ident, $k:ident, $d:ident, all($t:ty)) => {{ let _ = $d; $crate::ServiceProviderServiceExtensions::get_services::<$t>($p)? }};
    (@resolve $p:ident, $k:ident, $d:ident, keyed($t:ty, $key:expr)) => {{ let _ = $d; $crate::ServiceProviderServiceExtensions::get_required_keyed_service::<$t>($p, $d.as_ref().expect("keyed parameter").identifier.service_key.as_ref())? }};
    (@resolve $p:ident, $k:ident, $d:ident, inherited($t:ty)) => {{ let _ = $d; $crate::ServiceProviderServiceExtensions::get_required_keyed_service::<$t>($p, $k)? }};
    (@resolve $p:ident, $k:ident, $d:ident, typed_key($t:ty)) => {{
        let _ = $d;
        $k.and_then(|key| key.downcast_ref::<$t>()).cloned()
            .ok_or($crate::ServiceError::TypeMismatch($crate::ServiceType::of::<$t>()))?
    }};
    (@resolve $p:ident, $k:ident, $d:ident, key()) => {{ let _ = $d; $k.cloned() }};
}
