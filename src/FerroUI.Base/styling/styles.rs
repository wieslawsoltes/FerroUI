use super::{style_ptr_eq, ControlTheme, IStyle, SelectorMatchResult, Style, StyleHostRef, ThemeVariant};
use crate::collections::{
    CollectionChangedHandler, FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs,
    ResetBehavior,
};
use crate::controls::{
    IResourceNode, IResourceProvider, ResourceDictionary, ResourceHostRef, ResourceKey, ResourceValue,
    WeakResourceHost,
};
use crate::property_store::FrameType;
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{
    ferro_class, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt, ObjectType, Ref, StyledElement,
    Upcast,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A style that consists of a number of child styles.
#[repr(C)]
pub struct Styles {
    base: FerroObject,
    styles: FerroList<Rc<dyn IStyle>>,
    owner: RefCell<Option<WeakResourceHost>>,
    resources: RefCell<Option<Ref<ResourceDictionary>>>,
    collection_changed: HandlerList<CollectionChangedHandler<Rc<dyn IStyle>>>,
    owner_changed: HandlerList<dyn Fn()>,
}

ferro_class!(Styles: FerroObject);
crate::ferro_class_info!(Styles {
    new: Styles::new,
    interfaces: [Rc<dyn IStyle>, Rc<dyn IResourceProvider>],
});

impl FerroObjectImpl for Styles {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        this.styles.set_reset_behavior(ResetBehavior::Remove);
        let weak = this.to_ref().downgrade();
        this.styles.add_collection_changed(Rc::new(
            move |e: &NotifyCollectionChangedEventArgs<'_, Rc<dyn IStyle>>| {
                if let Some(this) = weak.upgrade() {
                    this.on_collection_changed(e);
                }
            },
        ));
        this.styles.set_validator(Some(Rc::new(|item: &Rc<dyn IStyle>| {
            if let Some(theme) = item.as_object().and_then(|o| o.downcast_ref::<ControlTheme>()) {
                panic!(
                    "ControlTheme (for {}) cannot be added to a Styles collection.",
                    theme.target_type().map(|t| t.name()).unwrap_or_default()
                );
            }
        })));
    }
}

impl Styles {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            styles: FerroList::new(),
            owner: RefCell::new(None),
            resources: RefCell::new(None),
            collection_changed: HandlerList::new(),
            owner_changed: HandlerList::new(),
        }
    }

    /// Creates an empty styles collection.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates an empty styles collection that belongs to `owner`.
    pub fn with_owner(owner: impl Into<ResourceHostRef>) -> Ref<Self> {
        let this = Self::construct();
        *this.owner.borrow_mut() = Some(owner.into().downgrade());
        instantiate(this)
    }

    /// Raised when the collection changes. Disposing the returned handle
    /// unsubscribes.
    pub fn collection_changed(
        &self,
        handler: impl for<'a> Fn(&NotifyCollectionChangedEventArgs<'a, Rc<dyn IStyle>>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.collection_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.collection_changed.remove(token);
            }
        })
    }

    /// Raised when the owner of the collection changes.
    pub fn owner_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_owner_changed(Rc::new(handler))
    }

    fn subscribe_owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.owner_changed.add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.owner_changed.remove(token);
            }
        })
    }

    /// The number of styles.
    #[inline]
    pub fn count(&self) -> usize {
        self.styles.count()
    }

    /// The owner of the collection.
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

    /// The dictionary of style resources.
    pub fn resources(&self) -> Ref<ResourceDictionary> {
        if let Some(resources) = &*self.resources.borrow() {
            return resources.clone();
        }
        let resources = ResourceDictionary::new();
        self.set_resources(resources.clone());
        resources
    }

    pub fn set_resources(&self, value: Ref<ResourceDictionary>) {
        let current_owner = self.owner();

        if let Some(owner) = &current_owner {
            let old = self.resources.borrow().clone();
            if let Some(old) = old {
                old.remove_owner(owner);
            }
        }

        *self.resources.borrow_mut() = Some(value.clone());

        if let Some(owner) = &current_owner {
            value.add_owner(owner);
        }
    }

    /// Whether the collection or any of its styles has resources.
    pub fn has_resources(&self) -> bool {
        if self.resources.borrow().as_ref().is_some_and(|r| r.count() > 0) {
            return true;
        }

        if self.styles.is_empty() {
            return false;
        }

        self.styles
            .snapshot()
            .iter()
            .any(|i| i.as_resource_provider().is_some_and(|provider| provider.has_resources()))
    }

    /// The style at `index`. Panics if out of range.
    pub fn get(&self, index: usize) -> Rc<dyn IStyle> {
        self.styles.get(index)
    }

    /// Replaces the style at `index`.
    ///
    /// The replacement is notified as a removal followed by an insertion.
    pub fn set(&self, index: usize, value: impl Into<Rc<dyn IStyle>>) {
        self.styles.remove_at(index);
        self.styles.insert(index, value.into());
    }

    /// A snapshot of the styles.
    #[inline]
    pub fn snapshot(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        self.styles.snapshot()
    }

    /// Tries to find a resource in the collection's resources and then in
    /// its styles, last style first.
    pub fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let resources = self.resources.borrow().clone();
        if let Some(resources) = resources {
            if let Some(value) = resources.try_get_resource(key, theme) {
                return Some(value);
            }
        }

        if !self.styles.is_empty() {
            for style in self.styles.snapshot().iter().rev() {
                if let Some(value) = style.try_get_resource(key, theme) {
                    return Some(value);
                }
            }
        }

        None
    }

    pub fn add_range(&self, items: impl IntoIterator<Item = Rc<dyn IStyle>>) {
        self.styles.add_range(items)
    }

    pub fn insert_range(&self, index: usize, items: impl IntoIterator<Item = Rc<dyn IStyle>>) {
        self.styles.insert_range(index, items)
    }

    pub fn move_item(&self, old_index: usize, new_index: usize) {
        self.styles.move_item(old_index, new_index)
    }

    pub fn move_range(&self, old_index: usize, count: usize, new_index: usize) {
        self.styles.move_range(old_index, count, new_index)
    }

    pub fn remove_all(&self, items: impl IntoIterator<Item = Rc<dyn IStyle>>) {
        for item in items {
            self.remove(item);
        }
    }

    pub fn remove_range(&self, index: usize, count: usize) {
        self.styles.remove_range(index, count)
    }

    /// The index of a style, if it is in the collection.
    pub fn index_of(&self, item: impl Into<Rc<dyn IStyle>>) -> Option<usize> {
        let item = item.into();
        self.styles.snapshot().iter().position(|s| style_ptr_eq(s, &item))
    }

    pub fn insert(&self, index: usize, item: impl Into<Rc<dyn IStyle>>) {
        self.styles.insert(index, item.into())
    }

    pub fn remove_at(&self, index: usize) {
        self.styles.remove_at(index)
    }

    /// Adds a style to the collection.
    pub fn add(&self, item: impl Into<Rc<dyn IStyle>>) {
        self.styles.add(item.into())
    }

    /// Removes all styles from the collection.
    pub fn clear(&self) {
        self.styles.clear()
    }

    pub fn contains(&self, item: impl Into<Rc<dyn IStyle>>) -> bool {
        self.index_of(item).is_some()
    }

    /// Removes a style from the collection. Returns whether it was present.
    pub fn remove(&self, item: impl Into<Rc<dyn IStyle>>) -> bool {
        match self.index_of(item) {
            Some(index) => {
                self.styles.remove_at(index);
                true
            }
            None => false,
        }
    }

    /// Adds an owner to the collection and the resource providers in it.
    pub fn add_owner(&self, owner: &ResourceHostRef) {
        if self.owner().is_some() {
            panic!("The Styles already has a owner.");
        }

        self.set_owner(Some(owner));
        let resources = self.resources.borrow().clone();
        if let Some(resources) = resources {
            resources.add_owner(owner);
        }

        for child in self.styles.snapshot().iter() {
            if let Some(r) = child.as_resource_provider() {
                r.add_owner(owner);
            }
        }
    }

    /// Removes the owner of the collection and the resource providers in it.
    pub fn remove_owner(&self, owner: &ResourceHostRef) {
        if self.owner().as_ref() == Some(owner) {
            self.set_owner(None);
            let resources = self.resources.borrow().clone();
            if let Some(resources) = resources {
                resources.remove_owner(owner);
            }

            for child in self.styles.snapshot().iter() {
                if let Some(r) = child.as_resource_provider() {
                    r.remove_owner(owner);
                }
            }
        }
    }

    /// Attaches the styles of the collection (but not of nested collections)
    /// that match `target`.
    #[allow(dead_code)]
    pub(crate) fn try_attach(&self, target: &StyledElement, host: Option<&StyleHostRef>) -> SelectorMatchResult {
        let mut result = SelectorMatchResult::NeverThisType;

        for s in self.styles.snapshot().iter() {
            let Some(style) = s.as_object().and_then(|o| o.downcast_ref::<Style>()) else { continue };

            let r = style.try_attach(target, host, FrameType::Style);
            if r > result {
                result = r;
            }
        }

        result
    }

    fn internal_add(items: &[Rc<dyn IStyle>], owner: Option<&ResourceHostRef>) {
        if let Some(owner) = owner {
            for item in items {
                if let Some(provider) = item.as_resource_provider() {
                    provider.add_owner(owner);
                }
            }

            if let Some(host) = owner.as_style_host() {
                host.styles_added(items);
            }
        }
    }

    fn internal_remove(items: &[Rc<dyn IStyle>], owner: Option<&ResourceHostRef>) {
        if let Some(owner) = owner {
            for item in items {
                if let Some(provider) = item.as_resource_provider() {
                    provider.remove_owner(owner);
                }
            }

            if let Some(host) = owner.as_style_host() {
                host.styles_removed(items);
            }
        }
    }

    fn on_collection_changed(&self, e: &NotifyCollectionChangedEventArgs<'_, Rc<dyn IStyle>>) {
        if e.action == NotifyCollectionChangedAction::Reset {
            panic!("Reset should not be called on Styles.");
        }

        let current_owner = self.owner();

        match e.action {
            NotifyCollectionChangedAction::Add => Self::internal_add(e.new_items, current_owner.as_ref()),
            NotifyCollectionChangedAction::Remove => Self::internal_remove(e.old_items, current_owner.as_ref()),
            NotifyCollectionChangedAction::Replace => {
                Self::internal_remove(e.old_items, current_owner.as_ref());
                Self::internal_add(e.new_items, current_owner.as_ref());
            }
            _ => {}
        }

        if !self.collection_changed.is_empty() {
            for (_, handler) in self.collection_changed.snapshot().iter() {
                handler(e);
            }
        }
    }
}

/// The style and resource provider interfaces of a styles collection object.
struct StylesHandle(Ref<Styles>);

impl IResourceNode for StylesHandle {
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

impl IResourceProvider for StylesHandle {
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

impl IStyle for StylesHandle {
    fn children(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        self.0.styles.snapshot()
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some((*self.0).upcast())
    }

    fn as_resource_provider(&self) -> Option<&dyn IResourceProvider> {
        Some(self)
    }
}

/// Converts a handle of a styles collection (or a class derived from it)
/// into a style handle.
pub fn styles_as_style<T: ObjectType + Upcast<Styles>>(styles: &Ref<T>) -> Rc<dyn IStyle> {
    Rc::new(StylesHandle(styles.clone().upcast()))
}

impl From<Ref<Styles>> for Rc<dyn IStyle> {
    fn from(value: Ref<Styles>) -> Self {
        Rc::new(StylesHandle(value))
    }
}

impl From<Ref<Styles>> for Rc<dyn IResourceProvider> {
    fn from(value: Ref<Styles>) -> Self {
        Rc::new(StylesHandle(value))
    }
}

impl From<&Ref<Styles>> for Rc<dyn IResourceProvider> {
    fn from(value: &Ref<Styles>) -> Self {
        Rc::new(StylesHandle(value.clone()))
    }
}

impl From<&Ref<Styles>> for Rc<dyn IStyle> {
    fn from(value: &Ref<Styles>) -> Self {
        Rc::new(StylesHandle(value.clone()))
    }
}
