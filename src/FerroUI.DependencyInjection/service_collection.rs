//! `ServiceCollection.cs`, `IServiceCollection.cs`, `ServiceCollectionServiceExtensions.cs`,
//! `ServiceCollectionKeyedServiceExtensions.cs`, `Extensions/ServiceCollectionDescriptorExtensions.cs`
//! and `ServiceCollectionContainerBuilderExtensions.cs`.

use crate::*;
use std::rc::Rc;

/// Ordered registrations; building a provider snapshots this collection.
#[derive(Clone, Default)]
pub struct ServiceCollection {
    descriptors: Vec<ServiceDescriptor>,
    read_only: bool,
}
pub trait IServiceCollection {
    fn descriptors(&self) -> &[ServiceDescriptor];
    fn add(&mut self, descriptor: ServiceDescriptor) -> &mut Self;
}
impl IServiceCollection for ServiceCollection {
    fn descriptors(&self) -> &[ServiceDescriptor] {
        &self.descriptors
    }
    fn add(&mut self, descriptor: ServiceDescriptor) -> &mut Self {
        self.add(descriptor)
    }
}
impl ServiceCollection {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn descriptors(&self) -> &[ServiceDescriptor] {
        &self.descriptors
    }
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
    pub fn make_read_only(&mut self) {
        self.read_only = true;
    }
    fn ensure_mutable(&self) {
        assert!(!self.read_only, "The service collection is read-only");
    }
    pub fn add(&mut self, descriptor: ServiceDescriptor) -> &mut Self {
        self.ensure_mutable();
        self.descriptors.push(descriptor);
        self
    }
    pub fn add_sequence(
        &mut self,
        descriptors: impl IntoIterator<Item = ServiceDescriptor>,
    ) -> &mut Self {
        self.ensure_mutable();
        self.descriptors.extend(descriptors);
        self
    }
    pub fn insert(&mut self, index: usize, descriptor: ServiceDescriptor) {
        self.ensure_mutable();
        self.descriptors.insert(index, descriptor);
    }
    pub fn remove_at(&mut self, index: usize) -> ServiceDescriptor {
        self.ensure_mutable();
        self.descriptors.remove(index)
    }
    pub fn clear(&mut self) {
        self.ensure_mutable();
        self.descriptors.clear();
    }
    pub fn try_add(&mut self, descriptor: ServiceDescriptor) -> &mut Self {
        self.ensure_mutable();
        if !self
            .descriptors
            .iter()
            .any(|d| d.identifier == descriptor.identifier)
        {
            self.add(descriptor);
        }
        self
    }
    pub fn try_add_enumerable(&mut self, descriptor: ServiceDescriptor) -> &mut Self {
        self.ensure_mutable();
        let implementation = descriptor
            .implementation_type
            .expect("TryAddEnumerable needs with_implementation::<I>()");
        assert!(
            implementation != descriptor.service_type()
                && implementation != ServiceType::of::<dyn std::any::Any>(),
            "Implementation type is indistinguishable from the service type"
        );
        if !self.descriptors.iter().any(|d| {
            d.identifier == descriptor.identifier && d.implementation_type == Some(implementation)
        }) {
            self.add(descriptor);
        }
        self
    }
    /// Removes the first matching registration, then appends the replacement, as upstream.
    pub fn replace(&mut self, descriptor: ServiceDescriptor) -> &mut Self {
        self.ensure_mutable();
        if let Some(i) = self
            .descriptors
            .iter()
            .position(|d| d.identifier == descriptor.identifier)
        {
            self.descriptors.remove(i);
        }
        self.add(descriptor)
    }
    pub fn remove_all<S: ?Sized + 'static>(&mut self) -> &mut Self {
        self.remove_all_keyed::<S>(None)
    }
    pub fn remove_all_keyed<S: ?Sized + 'static>(&mut self, key: Option<ServiceKey>) -> &mut Self {
        self.ensure_mutable();
        let id = ServiceIdentifier::keyed::<S>(key);
        self.descriptors.retain(|d| d.identifier != id);
        self
    }
    pub fn iter(&self) -> std::slice::Iter<'_, ServiceDescriptor> {
        self.descriptors.iter()
    }
    pub fn contains(&self, descriptor: &ServiceDescriptor) -> bool {
        self.index_of(descriptor).is_some()
    }
    pub fn index_of(&self, descriptor: &ServiceDescriptor) -> Option<usize> {
        self.descriptors
            .iter()
            .position(|d| Rc::ptr_eq(&d.identity, &descriptor.identity))
    }
    pub fn remove(&mut self, descriptor: &ServiceDescriptor) -> bool {
        self.ensure_mutable();
        if let Some(index) = self.index_of(descriptor) {
            self.descriptors.remove(index);
            true
        } else {
            false
        }
    }
    pub fn copy_to(&self, target: &mut [ServiceDescriptor], index: usize) {
        target[index..index + self.len()].clone_from_slice(&self.descriptors);
    }
    pub fn build_service_provider(
        &self,
        options: ServiceProviderOptions,
    ) -> ServiceResult<ServiceProvider> {
        ServiceProvider::build(self.descriptors.clone(), options)
    }
    pub fn add_singleton_instance<S: ?Sized + 'static>(&mut self, value: Rc<S>) -> &mut Self {
        self.add(ServiceDescriptor::instance(value))
    }
    pub fn add_keyed_singleton_instance<S: ?Sized + 'static>(
        &mut self,
        key: Option<ServiceKey>,
        value: Rc<S>,
    ) -> &mut Self {
        self.add(ServiceDescriptor::instance(value).with_key(key))
    }
}
macro_rules! registrations {
    ($add:ident, $keyed:ident, $lifetime:ident) => {
        impl ServiceCollection {
            pub fn $add<S: ?Sized + 'static, F>(&mut self, factory: F) -> &mut Self
            where
                F: Fn(&ServiceProvider) -> ServiceResult<Rc<S>> + 'static,
            {
                self.add(ServiceDescriptor::new(ServiceLifetime::$lifetime, factory))
            }
            pub fn $keyed<S: ?Sized + 'static, F>(
                &mut self,
                key: Option<ServiceKey>,
                factory: F,
            ) -> &mut Self
            where
                F: Fn(&ServiceProvider, Option<&ServiceKey>) -> ServiceResult<Rc<S>> + 'static,
            {
                self.add(ServiceDescriptor::keyed(
                    ServiceLifetime::$lifetime,
                    key,
                    factory,
                ))
            }
        }
    };
}
registrations!(add_singleton, add_keyed_singleton, Singleton);
registrations!(add_scoped, add_keyed_scoped, Scoped);
registrations!(add_transient, add_keyed_transient, Transient);

impl std::ops::Index<usize> for ServiceCollection {
    type Output = ServiceDescriptor;
    fn index(&self, index: usize) -> &Self::Output {
        &self.descriptors[index]
    }
}
impl std::ops::IndexMut<usize> for ServiceCollection {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.ensure_mutable();
        &mut self.descriptors[index]
    }
}
impl<'a> IntoIterator for &'a ServiceCollection {
    type Item = &'a ServiceDescriptor;
    type IntoIter = std::slice::Iter<'a, ServiceDescriptor>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
