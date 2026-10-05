use super::{
    IResourceNode, IResourceProvider, ResourceHostRef, ResourceKey, ResourceValue, ResourcesChangedEventArgs,
    WeakResourceHost,
};
use crate::reactive::{Disposable, IDisposable};
use crate::styling::ThemeVariant;
use crate::utilities::HandlerList;
use crate::{ferro_class, FerroObject, FerroObjectImpl, ObjectType, Ref, Upcast};
use std::cell::RefCell;
use std::rc::Rc;

/// Base implementation for resource providers.
#[repr(C)]
pub struct ResourceProvider {
    base: FerroObject,
    owner: RefCell<Option<WeakResourceHost>>,
    owner_changed: HandlerList<dyn Fn()>,
}

ferro_class! {
    ResourceProvider: FerroObject, virtuals ResourceProviderImpl: FerroObjectImpl {
        /// Whether the provider has resources.
        fn has_resources(this) -> bool;
        /// Tries to find a resource within the provider.
        fn try_get_resource(this, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue>;
        /// Handles when the owner is added. The base method notifies the
        /// owner about its resources changing, if the provider has any.
        fn on_add_owner(this, owner: &ResourceHostRef);
        /// Handles when the owner is removed. The base method notifies the
        /// owner about its resources changing, if the provider has any.
        fn on_remove_owner(this, owner: &ResourceHostRef);
    }
}

crate::ferro_class_info!(ResourceProvider { interfaces: [Rc<dyn IResourceProvider>] });

impl FerroObjectImpl for ResourceProvider {}

impl ResourceProviderImpl for ResourceProvider {
    fn has_resources(_this: &Self) -> bool {
        false
    }

    fn try_get_resource(_this: &Self, _key: &ResourceKey, _theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        None
    }

    fn on_add_owner(this: &Self, owner: &ResourceHostRef) {
        if this.has_resources() {
            owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
        }
    }

    fn on_remove_owner(this: &Self, owner: &ResourceHostRef) {
        if this.has_resources() {
            owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
        }
    }
}

impl ResourceProvider {
    /// Creates the class data of a provider without an owner.
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), owner: RefCell::new(None), owner_changed: HandlerList::new() }
    }

    /// Creates the class data of a provider that belongs to `owner`.
    pub fn construct_with_owner(owner: &ResourceHostRef) -> Self {
        Self {
            base: FerroObject::construct(),
            owner: RefCell::new(Some(owner.downgrade())),
            owner_changed: HandlerList::new(),
        }
    }

    /// The owner of the resource provider.
    pub fn owner(&self) -> Option<ResourceHostRef> {
        let owner = self.owner.borrow().clone();
        owner.and_then(|o| o.upgrade())
    }

    fn set_owner(&self, value: Option<&ResourceHostRef>) {
        let old = self.owner();
        if old.as_ref() != value {
            *self.owner.borrow_mut() = value.map(ResourceHostRef::downgrade);
            if !self.owner_changed.is_empty() {
                for (_, handler) in self.owner_changed.snapshot().iter() {
                    handler();
                }
            }
        }
    }

    /// Raised when the owner of the resource provider changes.
    pub fn owner_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_owner_changed(Rc::new(handler))
    }

    pub(crate) fn subscribe_owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.owner_changed.add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.owner_changed.remove(token);
            }
        })
    }

    /// Notifies the owner, if any, that the resources of the provider have
    /// changed.
    pub fn raise_resources_changed(&self) {
        if let Some(owner) = self.owner() {
            owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
        }
    }

    /// Adds an owner to the resource provider.
    pub fn add_owner(&self, owner: &ResourceHostRef) {
        if self.owner().is_some() {
            panic!("The ResourceDictionary already has a parent.");
        }
        self.set_owner(Some(owner));
        self.on_add_owner(owner);
    }

    /// Removes a resource provider owner.
    pub fn remove_owner(&self, owner: &ResourceHostRef) {
        if self.owner().as_ref() == Some(owner) {
            self.set_owner(None);
            self.on_remove_owner(owner);
        }
    }
}

/// The resource provider interface of a provider object.
struct ResourceProviderHandle(Ref<ResourceProvider>);

impl IResourceNode for ResourceProviderHandle {
    fn reference_id(&self) -> *const () {
        let object: &FerroObject = (*self.0).upcast();
        object as *const FerroObject as *const ()
    }

    fn has_resources(&self) -> bool {
        self.0.has_resources()
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.0.try_get_resource(key, theme)
    }
}

impl IResourceProvider for ResourceProviderHandle {
    fn owner(&self) -> Option<ResourceHostRef> {
        self.0.owner()
    }

    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.subscribe_owner_changed(handler)
    }

    fn add_owner(&self, owner: &ResourceHostRef) {
        self.0.add_owner(owner)
    }

    fn remove_owner(&self, owner: &ResourceHostRef) {
        self.0.remove_owner(owner)
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some((*self.0).upcast())
    }
}

impl<T: ObjectType + Upcast<ResourceProvider>> From<Ref<T>> for Rc<dyn IResourceProvider> {
    fn from(value: Ref<T>) -> Self {
        Rc::new(ResourceProviderHandle(value.upcast()))
    }
}

impl<T: ObjectType + Upcast<ResourceProvider>> From<&Ref<T>> for Rc<dyn IResourceProvider> {
    fn from(value: &Ref<T>) -> Self {
        Rc::new(ResourceProviderHandle(value.clone().upcast()))
    }
}
