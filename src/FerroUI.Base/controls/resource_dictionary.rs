use super::resource_key::ResourceKeyBuildHasher;
use super::{
    resource_provider_ptr_eq, IDeferredContent, IResourceDictionary, IResourceNode, IResourceProvider,
    IThemeVariantProvider, ResourceHostRef, ResourceKey, ResourceProvider, ResourceProviderImpl, ResourceValue,
    ResourcesChangedEventArgs,
};
use crate::collections::{FerroDictionary, FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, ResetBehavior};
use crate::metadata::IServiceProvider;
use crate::reactive::IDisposable;
use crate::styling::ThemeVariant;
use crate::{ferro_class, instantiate, BoxedValue, FerroObject, FerroObjectImpl, ObjectType, Ref, Upcast};
use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone)]
enum ResourceItem {
    Value(ResourceValue),
    Deferred(Rc<dyn IDeferredContent>),
    NotSharedDeferred(Rc<dyn IDeferredContent>),
}

struct DeferredItem<F: Fn(Option<&Rc<dyn IServiceProvider>>) -> Option<BoxedValue>>(F);

impl<F: Fn(Option<&Rc<dyn IServiceProvider>>) -> Option<BoxedValue>> IDeferredContent for DeferredItem<F> {
    fn build(&self, service_provider: Option<&Rc<dyn IServiceProvider>>) -> Option<BoxedValue> {
        (self.0)(service_provider)
    }
}

/// An indexed dictionary of resources.
#[repr(C)]
pub struct ResourceDictionary {
    base: ResourceProvider,
    last_deferred_item_key: RefCell<Option<ResourceKey>>,
    inner: RefCell<HashMap<ResourceKey, ResourceItem, ResourceKeyBuildHasher>>,
    merged_dictionaries: OnceCell<FerroList<Rc<dyn IResourceProvider>>>,
    theme_dictionaries: OnceCell<FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>>>,
    key: RefCell<Option<ThemeVariant>>,
}

ferro_class!(ResourceDictionary: ResourceProvider);
crate::ferro_class_info!(ResourceDictionary {
    new: ResourceDictionary::new,
    interfaces: [Rc<dyn IResourceProvider>, Rc<dyn IResourceDictionary>, Rc<dyn IThemeVariantProvider>],
});

impl FerroObjectImpl for ResourceDictionary {}

impl ResourceProviderImpl for ResourceDictionary {
    fn has_resources(this: &Self) -> bool {
        if !this.inner.borrow().is_empty() {
            return true;
        }
        if let Some(merged) = this.merged_dictionaries.get() {
            if !merged.is_empty() {
                return merged.snapshot().iter().any(|i| i.has_resources());
            }
        }
        false
    }

    fn try_get_resource(this: &Self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        if let Some(value) = this.try_get_value(key) {
            return Some(value);
        }

        if this.theme_dictionaries.get().is_some() {
            if let Some(theme) = theme {
                if *theme != ThemeVariant::default() {
                    if let Some(provider) = this.theme_dictionary(theme) {
                        if let Some(value) = provider.try_get_resource(key, Some(theme)) {
                            return Some(value);
                        }
                    }

                    let mut theme_inherit = theme.inherit_variant();
                    while let Some(inherit) = theme_inherit {
                        if let Some(provider) = this.theme_dictionary(&inherit) {
                            if let Some(value) = provider.try_get_resource(key, Some(theme)) {
                                return Some(value);
                            }
                        }
                        theme_inherit = inherit.inherit_variant();
                    }
                }
            }

            if let Some(provider) = this.theme_dictionary(&ThemeVariant::default()) {
                if let Some(value) = provider.try_get_resource(key, theme) {
                    return Some(value);
                }
            }
        }

        if let Some(merged) = this.merged_dictionaries.get() {
            if !merged.is_empty() {
                for provider in merged.snapshot().iter().rev() {
                    if let Some(value) = provider.try_get_resource(key, theme) {
                        return Some(value);
                    }
                }
            }
        }

        None
    }

    fn on_add_owner(this: &Self, owner: &ResourceHostRef) {
        let mut has_resources = !this.inner.borrow().is_empty();

        if let Some(merged) = this.merged_dictionaries.get() {
            for i in merged.snapshot().iter() {
                i.add_owner(owner);
                has_resources |= i.has_resources();
            }
        }

        for (_, i) in this.theme_dictionaries_snapshot() {
            i.add_owner(owner);
            has_resources |= i.has_resources();
        }

        if has_resources {
            owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
        }
    }

    fn on_remove_owner(this: &Self, owner: &ResourceHostRef) {
        let mut has_resources = !this.inner.borrow().is_empty();

        if let Some(merged) = this.merged_dictionaries.get() {
            for i in merged.snapshot().iter() {
                i.remove_owner(owner);
                has_resources |= i.has_resources();
            }
        }

        for (_, i) in this.theme_dictionaries_snapshot() {
            i.remove_owner(owner);
            has_resources |= i.has_resources();
        }

        if has_resources {
            owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
        }
    }
}

impl ResourceDictionary {
    fn construct_from(base: ResourceProvider) -> Self {
        Self {
            base,
            last_deferred_item_key: RefCell::new(None),
            inner: RefCell::new(HashMap::default()),
            merged_dictionaries: OnceCell::new(),
            theme_dictionaries: OnceCell::new(),
            key: RefCell::new(None),
        }
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self::construct_from(ResourceProvider::construct())
    }

    /// Creates the class data of a dictionary that belongs to `owner`.
    pub fn construct_with_owner(owner: &ResourceHostRef) -> Self {
        Self::construct_from(ResourceProvider::construct_with_owner(owner))
    }

    /// Creates an empty resource dictionary.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates an empty resource dictionary that belongs to `owner`.
    pub fn with_owner(owner: impl Into<ResourceHostRef>) -> Ref<Self> {
        instantiate(Self::construct_with_owner(&owner.into()))
    }

    /// The number of resources in the dictionary itself.
    pub fn count(&self) -> usize {
        self.inner.borrow().len()
    }

    /// Gets a resource of the dictionary itself; `None` (a null value) if the
    /// key is not present.
    pub fn get(&self, key: &ResourceKey) -> ResourceValue {
        self.try_get_value(key).flatten()
    }

    /// Adds or replaces a resource.
    pub fn set(&self, key: impl Into<ResourceKey>, value: ResourceValue) {
        self.inner.borrow_mut().insert(key.into(), ResourceItem::Value(value));
        self.raise_resources_changed();
    }

    /// The keys of the resources in the dictionary itself.
    pub fn keys(&self) -> Vec<ResourceKey> {
        self.inner.borrow().keys().cloned().collect()
    }

    /// The child resource dictionaries.
    ///
    /// The list is returned by handle: clones refer to the same list.
    pub fn merged_dictionaries(&self) -> FerroList<Rc<dyn IResourceProvider>> {
        self.merged_dictionaries_ref().clone()
    }

    fn merged_dictionaries_ref(&self) -> &FerroList<Rc<dyn IResourceProvider>> {
        self.merged_dictionaries.get_or_init(|| {
            let list = FerroList::new();
            list.set_reset_behavior(ResetBehavior::Remove);
            let weak = self.to_ref().downgrade();
            list.add_collection_changed(Rc::new(
                move |e: &NotifyCollectionChangedEventArgs<'_, Rc<dyn IResourceProvider>>| {
                    let Some(this) = weak.upgrade() else { return };
                    let added = |items: &[Rc<dyn IResourceProvider>]| {
                        for x in items {
                            if let Some(owner) = this.owner() {
                                x.add_owner(&owner);
                            }
                        }
                    };
                    let removed = |items: &[Rc<dyn IResourceProvider>]| {
                        for x in items {
                            if let Some(owner) = this.owner() {
                                x.remove_owner(&owner);
                            }
                        }
                    };
                    match e.action {
                        NotifyCollectionChangedAction::Add => added(e.new_items),
                        NotifyCollectionChangedAction::Remove => removed(e.old_items),
                        NotifyCollectionChangedAction::Replace => {
                            removed(e.old_items);
                            added(e.new_items);
                        }
                        NotifyCollectionChangedAction::Reset => panic!("Dictionary reset not supported"),
                        NotifyCollectionChangedAction::Move => {}
                    }
                },
            ));
            list
        })
    }

    /// Adds a child resource dictionary.
    pub fn add_merged_dictionary(&self, provider: impl Into<Rc<dyn IResourceProvider>>) {
        self.merged_dictionaries().add(provider.into());
    }

    /// Removes a child resource dictionary. Returns whether it was present.
    pub fn remove_merged_dictionary(&self, provider: &Rc<dyn IResourceProvider>) -> bool {
        let list = self.merged_dictionaries();
        let index = list.snapshot().iter().position(|p| resource_provider_ptr_eq(p, provider));
        match index {
            Some(index) => {
                list.remove_at(index);
                true
            }
            None => false,
        }
    }

    /// The resource providers used for specific theme variants.
    ///
    /// The dictionary is returned by handle: clones refer to the same
    /// dictionary. A provider added to it gets the owner of this dictionary;
    /// a removed one loses it.
    pub fn theme_dictionaries(&self) -> FerroDictionary<ThemeVariant, Rc<dyn IThemeVariantProvider>> {
        self.theme_dictionaries
            .get_or_init(|| {
                let dictionary = FerroDictionary::with_capacity(2);
                let weak = self.to_ref().downgrade();
                dictionary.add_collection_changed(Rc::new(
                    move |e: &NotifyCollectionChangedEventArgs<'_, (ThemeVariant, Rc<dyn IThemeVariantProvider>)>| {
                        let Some(this) = weak.upgrade() else { return };
                        let added = |items: &[(ThemeVariant, Rc<dyn IThemeVariantProvider>)]| {
                            for (_, x) in items {
                                if let Some(owner) = this.owner() {
                                    x.add_owner(&owner);
                                }
                            }
                        };
                        let removed = |items: &[(ThemeVariant, Rc<dyn IThemeVariantProvider>)]| {
                            for (_, x) in items {
                                if let Some(owner) = this.owner() {
                                    x.remove_owner(&owner);
                                }
                            }
                        };
                        match e.action {
                            NotifyCollectionChangedAction::Add => added(e.new_items),
                            NotifyCollectionChangedAction::Remove => removed(e.old_items),
                            NotifyCollectionChangedAction::Replace | NotifyCollectionChangedAction::Move => {
                                removed(e.old_items);
                                added(e.new_items);
                            }
                            NotifyCollectionChangedAction::Reset => panic!("Dictionary reset not supported"),
                        }
                    },
                ));
                dictionary
            })
            .clone()
    }

    /// A snapshot of the resource providers used for specific theme variants.
    pub fn theme_dictionaries_snapshot(&self) -> Vec<(ThemeVariant, Rc<dyn IThemeVariantProvider>)> {
        match self.theme_dictionaries.get() {
            Some(dictionaries) if !dictionaries.is_empty() => dictionaries.to_vec(),
            _ => Vec::new(),
        }
    }

    /// The resource provider used for the given theme variant.
    pub fn theme_dictionary(&self, variant: &ThemeVariant) -> Option<Rc<dyn IThemeVariantProvider>> {
        self.theme_dictionaries.get().and_then(|d| d.try_get_value(variant))
    }

    /// Adds a resource provider for a theme variant. Panics if the variant
    /// already has one.
    pub fn add_theme_dictionary(&self, variant: ThemeVariant, provider: impl Into<Rc<dyn IThemeVariantProvider>>) {
        self.theme_dictionaries().add(variant, provider.into());
    }

    /// Adds or replaces the resource provider of a theme variant.
    pub fn set_theme_dictionary(&self, variant: ThemeVariant, provider: impl Into<Rc<dyn IThemeVariantProvider>>) {
        self.theme_dictionaries().set(variant, provider.into());
    }

    /// Removes the resource provider of a theme variant. Returns whether it
    /// was present.
    pub fn remove_theme_dictionary(&self, variant: &ThemeVariant) -> bool {
        self.theme_dictionaries.get().is_some_and(|d| d.remove(variant))
    }

    /// The key that is used in the theme dictionaries of the containing
    /// resource dictionary.
    pub fn key(&self) -> Option<ThemeVariant> {
        self.key.borrow().clone()
    }

    pub fn set_key(&self, value: Option<ThemeVariant>) {
        *self.key.borrow_mut() = value;
    }

    /// Adds a resource. Panics if the key is already present.
    pub fn add(&self, key: impl Into<ResourceKey>, value: ResourceValue) {
        self.add_item(key.into(), ResourceItem::Value(value));
    }

    /// Adds a resource value. Panics if the key is already present.
    pub fn add_value<T: PartialEq + 'static>(&self, key: impl Into<ResourceKey>, value: T) {
        self.add(key, Some(Rc::new(value)));
    }

    fn add_item(&self, key: ResourceKey, item: ResourceItem) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.contains_key(&key) {
                panic!("An item with the same key has already been added. Key: {key}");
            }
            inner.insert(key, item);
        }
        self.raise_resources_changed();
    }

    /// Adds a resource that is created by `factory` the first time it is
    /// requested.
    pub fn add_deferred_fn(
        &self,
        key: impl Into<ResourceKey>,
        factory: impl Fn(Option<&Rc<dyn IServiceProvider>>) -> Option<BoxedValue> + 'static,
    ) {
        self.add_item(key.into(), ResourceItem::Deferred(Rc::new(DeferredItem(factory))));
    }

    /// Adds a resource that is built from `deferred_content` the first time
    /// it is requested.
    pub fn add_deferred(&self, key: impl Into<ResourceKey>, deferred_content: Rc<dyn IDeferredContent>) {
        self.add_item(key.into(), ResourceItem::Deferred(deferred_content));
    }

    /// Adds a resource that is built from `deferred_content` every time it is
    /// requested.
    pub fn add_not_shared_deferred(&self, key: impl Into<ResourceKey>, deferred_content: Rc<dyn IDeferredContent>) {
        self.add_item(key.into(), ResourceItem::NotSharedDeferred(deferred_content));
    }

    /// Adds or replaces multiple resources, raising a single notification.
    pub fn set_items(&self, values: impl IntoIterator<Item = (ResourceKey, ResourceValue)>) {
        {
            let mut inner = self.inner.borrow_mut();
            for (key, value) in values {
                inner.insert(key, ResourceItem::Value(value));
            }
        }
        self.raise_resources_changed();
    }

    /// Removes all resources of the dictionary itself.
    pub fn clear(&self) {
        let cleared = {
            let mut inner = self.inner.borrow_mut();
            if inner.is_empty() {
                false
            } else {
                inner.clear();
                true
            }
        };
        if cleared {
            self.raise_resources_changed();
        }
    }

    /// Whether the dictionary itself contains the key.
    pub fn contains_key(&self, key: &ResourceKey) -> bool {
        self.inner.borrow().contains_key(key)
    }

    /// Removes a resource. Returns whether it was present.
    pub fn remove(&self, key: &ResourceKey) -> bool {
        let removed = self.inner.borrow_mut().remove(key).is_some();
        if removed {
            self.raise_resources_changed();
        }
        removed
    }

    /// Gets a resource of the dictionary itself, building it if it is
    /// deferred.
    pub fn try_get_value(&self, key: &ResourceKey) -> Option<ResourceValue> {
        let item = self.inner.borrow().get(key).cloned()?;
        let (deferred, shared) = match item {
            ResourceItem::Value(value) => return Some(value),
            ResourceItem::Deferred(deferred) => (deferred, true),
            ResourceItem::NotSharedDeferred(deferred) => (deferred, false),
        };

        // Avoid simple reentrancy, which could commonly occur on redefining
        // the resource.
        if self.last_deferred_item_key.borrow().as_ref() == Some(key) {
            return None;
        }

        struct Reset<'a>(&'a RefCell<Option<ResourceKey>>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                *self.0.borrow_mut() = None;
            }
        }

        *self.last_deferred_item_key.borrow_mut() = Some(key.clone());
        let _reset = Reset(&self.last_deferred_item_key);
        let value = deferred.build(None);
        if shared {
            self.inner.borrow_mut().insert(key.clone(), ResourceItem::Value(value.clone()));
        }
        Some(value)
    }

    /// Ensures that the dictionary can hold up to `capacity` resources
    /// without further expansion of its backing storage.
    pub fn ensure_capacity(&self, capacity: usize) {
        let mut inner = self.inner.borrow_mut();
        let additional = capacity.saturating_sub(inner.len());
        inner.reserve(additional);
    }

    /// Whether the key is present and its resource has not been built yet.
    pub fn contains_deferred_key(&self, key: &ResourceKey) -> bool {
        matches!(
            self.inner.borrow().get(key),
            Some(ResourceItem::Deferred(_) | ResourceItem::NotSharedDeferred(_))
        )
    }
}

/// The interfaces of a resource dictionary object.
struct ResourceDictionaryHandle(Ref<ResourceDictionary>);

impl IResourceNode for ResourceDictionaryHandle {
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

impl IResourceProvider for ResourceDictionaryHandle {
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

impl IThemeVariantProvider for ResourceDictionaryHandle {
    fn key(&self) -> Option<ThemeVariant> {
        self.0.key()
    }

    fn set_key(&self, value: Option<ThemeVariant>) {
        self.0.set_key(value)
    }
}

impl IResourceDictionary for ResourceDictionaryHandle {
    fn merged_dictionaries_snapshot(&self) -> Rc<Vec<Rc<dyn IResourceProvider>>> {
        self.0.merged_dictionaries().snapshot()
    }

    fn theme_dictionaries_snapshot(&self) -> Vec<(ThemeVariant, Rc<dyn IThemeVariantProvider>)> {
        self.0.theme_dictionaries_snapshot()
    }

    fn count(&self) -> usize {
        self.0.count()
    }

    fn add(&self, key: ResourceKey, value: ResourceValue) {
        self.0.add(key, value)
    }

    fn set(&self, key: ResourceKey, value: ResourceValue) {
        self.0.set(key, value)
    }

    fn contains_key(&self, key: &ResourceKey) -> bool {
        self.0.contains_key(key)
    }

    fn remove(&self, key: &ResourceKey) -> bool {
        self.0.remove(key)
    }

    fn try_get_value(&self, key: &ResourceKey) -> Option<ResourceValue> {
        self.0.try_get_value(key)
    }

    fn clear(&self) {
        self.0.clear()
    }
}

impl<T: ObjectType + Upcast<ResourceDictionary>> From<Ref<T>> for Rc<dyn IThemeVariantProvider> {
    fn from(value: Ref<T>) -> Self {
        Rc::new(ResourceDictionaryHandle(value.upcast()))
    }
}

impl<T: ObjectType + Upcast<ResourceDictionary>> From<&Ref<T>> for Rc<dyn IThemeVariantProvider> {
    fn from(value: &Ref<T>) -> Self {
        Rc::new(ResourceDictionaryHandle(value.clone().upcast()))
    }
}

impl<T: ObjectType + Upcast<ResourceDictionary>> From<Ref<T>> for Rc<dyn IResourceDictionary> {
    fn from(value: Ref<T>) -> Self {
        Rc::new(ResourceDictionaryHandle(value.upcast()))
    }
}
