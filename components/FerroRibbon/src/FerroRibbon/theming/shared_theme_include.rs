//! Source compiled-theme dictionaries are singletons. Native dictionaries keep
//! one weak owner: this native notification host fans changes out to weak users.
//! Concrete scope: FerroRibbon theme URIs only; no CLR/GC/runtime emulation.
use ferroui_base::{
    Ref, WeakRef,
    controls::{
        IResourceDictionary, IResourceHost, IResourceNode, IResourceProvider, ResourceDictionary,
        ResourceHostRef, ResourceKey, ResourceValue, ResourcesChangedEventArgs, WeakResourceHost,
    },
    data::core::ValueTypes,
    ferro_markup_type,
    reactive::{Disposable, IDisposable},
    styling::ThemeVariant,
    utilities::{HandlerList, Uri},
};
use ferroui_controls::Application;
use ferroui_markup_xaml::XamlLoadError;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

struct SharedHost {
    this: Weak<Self>,
    dictionary: WeakRef<ResourceDictionary>,
    owners: RefCell<Vec<(WeakResourceHost, usize)>>,
    changed: HandlerList<dyn Fn(&ResourcesChangedEventArgs)>,
}
impl SharedHost {
    fn new(dictionary: &Ref<ResourceDictionary>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            dictionary: dictionary.downgrade(),
            owners: RefCell::new(Vec::new()),
            changed: HandlerList::new(),
        })
    }
    fn attach(&self, owner: &ResourceHostRef) {
        let mut owners = self.owners.borrow_mut();
        owners.retain(|(w, _)| w.upgrade().is_some());
        if let Some((_, count)) = owners
            .iter_mut()
            .find(|(w, _)| w.upgrade().as_ref() == Some(owner))
        {
            *count = count
                .checked_add(1)
                .expect("resource owner reference count overflow");
            return;
        }
        owners.push((owner.downgrade(), 1));
        drop(owners);
        owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
    }
    fn detach(&self, owner: &ResourceHostRef) {
        let mut owners = self.owners.borrow_mut();
        let Some(index) = owners
            .iter()
            .position(|(w, _)| w.upgrade().as_ref() == Some(owner))
        else {
            return;
        };
        if owners[index].1 > 1 {
            owners[index].1 -= 1;
            return;
        }
        owners.remove(index);
        drop(owners);
        owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
    }
}
impl IResourceNode for SharedHost {
    fn has_resources(&self) -> bool {
        self.dictionary.upgrade().is_some_and(|d| d.has_resources())
    }
    fn try_get_resource(
        &self,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Option<ResourceValue> {
        self.dictionary
            .upgrade()
            .and_then(|d| d.try_get_resource(key, theme))
    }
}
impl IResourceHost for SharedHost {
    fn resources_changed(
        &self,
        handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        let token = self.changed.add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(host) = weak.upgrade() {
                host.changed.remove(token);
            }
        })
    }
    fn notify_hosted_resources_changed(&self, e: ResourcesChangedEventArgs) {
        for (_, handler) in self.changed.snapshot().iter() {
            handler(&e);
        }
        let owners = self
            .owners
            .borrow()
            .iter()
            .filter_map(|(w, _)| w.upgrade())
            .collect::<Vec<_>>();
        // Preserve one native change identity across paths: StyledElement dedups it.
        for owner in owners {
            owner.notify_hosted_resources_changed(e);
        }
    }
}
struct SharedDictionary {
    dictionary: Ref<ResourceDictionary>,
    host: Rc<SharedHost>,
}
// Deferred native markup captures its Application weakly. Reusing those
// factories after that Application ends would resolve against the old context.
// Keep source singleton identity within an Application, without retaining it.
struct DictionaryCache {
    application: Option<WeakRef<Application>>,
    dictionaries: RefCell<HashMap<String, Rc<SharedDictionary>>>,
    loading: RefCell<HashSet<String>>,
}
impl DictionaryCache {
    fn new(application: Option<&Ref<Application>>) -> Self {
        Self {
            application: application.map(Ref::downgrade),
            dictionaries: RefCell::new(HashMap::new()),
            loading: RefCell::new(HashSet::new()),
        }
    }
    fn matches(&self, application: Option<&Ref<Application>>) -> bool {
        match (&self.application, application) {
            (None, None) => true,
            (Some(old), Some(current)) => old.points_to(current),
            _ => false,
        }
    }
    fn current() -> Rc<Self> {
        let application = Application::current();
        DICTIONARIES.with(|slot| {
            let mut current = slot.borrow_mut();
            if current.matches(application.as_ref()) {
                return current.clone();
            }
            let next = Rc::new(Self::new(application.as_ref()));
            let old = std::mem::replace(&mut *current, next.clone());
            drop(current);
            // Drop stale factories outside the slot borrow: resource disposal
            // can invoke native callbacks. Active includes retain their values.
            drop(old);
            next
        })
    }
}
ferroui_base::ferro_thread_local! {
    static DICTIONARIES: RefCell<Rc<DictionaryCache>> = RefCell::new(Rc::new(DictionaryCache::new(None)));
}
struct Loading {
    cache: Rc<DictionaryCache>,
    key: String,
}
impl Drop for Loading {
    fn drop(&mut self) {
        self.cache.loading.borrow_mut().remove(&self.key);
    }
}
fn dictionary(uri: &Uri) -> Result<Rc<SharedDictionary>, XamlLoadError> {
    let key = uri.absolute_uri().to_owned();
    // Uri normalizes the authority to lowercase; paths remain case-sensitive.
    // Only these library-owned source families have source singleton semantics.
    if !matches!(
        key.as_str(),
        "ferres://ferroribbon/Themes/Shared.xaml"
            | "ferres://ferroribbon/Themes/Source/Items.xaml"
            | "ferres://ferroribbon/Themes/Source/Ribbon.xaml"
            | "ferres://ferroribbon/Themes/Backstage.xaml"
            | "ferres://ferroribbon/Themes/Source/Inputs.xaml"
            | "ferres://ferroribbon/Themes/Source/Chrome.xaml"
    ) {
        return Err(XamlLoadError::with_message(
            "Shared theme include requires a FerroRibbon theme URI.",
        ));
    }
    let cache = DictionaryCache::current();
    if let Some(found) = cache.dictionaries.borrow().get(&key).cloned() {
        return Ok(found);
    }
    if !cache.loading.borrow_mut().insert(key.clone()) {
        return Err(XamlLoadError::with_message(format!(
            "Cyclic shared theme include: {key}"
        )));
    }
    let _loading = Loading {
        cache: cache.clone(),
        key: key.clone(),
    };
    // These fixed library documents are already in the native compiled group.
    // Preserve loading before Application asset-service registration, and do not
    // retain a consumer's parent-stack service provider in shared resources.
    let value = crate::compiled_shared::try_load(None, &key)?.ok_or_else(|| {
        XamlLoadError::with_message(format!("Compiled shared theme was not registered: {key}"))
    })?;
    let loaded = ValueTypes::as_object(&*value)
        .and_then(|o| o.cast::<ResourceDictionary>())
        .ok_or_else(|| {
            XamlLoadError::with_message("Shared theme must be a native ResourceDictionary.")
        })?;
    let host = SharedHost::new(&loaded);
    loaded.add_owner(&ResourceHostRef::Other(host.clone()));
    let shared = Rc::new(SharedDictionary {
        dictionary: loaded,
        host,
    });
    cache.dictionaries.borrow_mut().insert(key, shared.clone());
    Ok(shared)
}

/// Markup implementation detail for source theme sharing, using native services.
#[doc(hidden)]
pub struct RibbonSharedThemeInclude {
    this: Weak<Self>,
    source: RefCell<Option<Uri>>,
    loaded: RefCell<Option<Rc<SharedDictionary>>>,
    owner: RefCell<Option<WeakResourceHost>>,
    owner_changed: HandlerList<dyn Fn()>,
}
impl PartialEq for RibbonSharedThemeInclude {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for RibbonSharedThemeInclude {}
impl RibbonSharedThemeInclude {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            source: RefCell::new(None),
            loaded: RefCell::new(None),
            owner: RefCell::new(None),
            owner_changed: HandlerList::new(),
        })
    }
    pub fn source(&self) -> Option<Uri> {
        self.source.borrow().clone()
    }
    pub fn set_source(&self, value: Option<Uri>) {
        *self.source.borrow_mut() = value;
    }
    fn shared(&self) -> Result<Rc<SharedDictionary>, XamlLoadError> {
        if let Some(found) = self.loaded.borrow().clone() {
            return Ok(found);
        }
        let source = self
            .source()
            .ok_or_else(|| XamlLoadError::with_message("Shared theme Source must be set."))?;
        if !source.is_absolute_uri() {
            return Err(XamlLoadError::with_message(
                "Shared theme requires an absolute library URI.",
            ));
        }
        let found = dictionary(&source)?;
        *self.loaded.borrow_mut() = Some(found.clone());
        Ok(found)
    }
    fn loaded(&self) -> Rc<SharedDictionary> {
        self.shared().unwrap_or_else(|e| panic!("{e}"))
    }
    pub fn try_loaded(&self) -> Result<Rc<dyn IResourceDictionary>, XamlLoadError> {
        self.shared().map(|s| s.dictionary.clone().into())
    }
    fn notify_owner_changed(&self) {
        for (_, handler) in self.owner_changed.snapshot().iter() {
            handler();
        }
    }
}
impl IResourceNode for RibbonSharedThemeInclude {
    fn has_resources(&self) -> bool {
        self.loaded().dictionary.has_resources()
    }
    fn try_get_resource(
        &self,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Option<ResourceValue> {
        self.loaded().dictionary.try_get_resource(key, theme)
    }
}
impl IResourceProvider for RibbonSharedThemeInclude {
    fn resource_dictionary(&self) -> Option<Rc<dyn IResourceDictionary>> {
        Some(self.loaded().dictionary.clone().into())
    }
    fn owner(&self) -> Option<ResourceHostRef> {
        self.owner
            .borrow()
            .as_ref()
            .and_then(WeakResourceHost::upgrade)
    }
    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.owner_changed.add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(include) = weak.upgrade() {
                include.owner_changed.remove(token);
            }
        })
    }
    fn add_owner(&self, owner: &ResourceHostRef) {
        assert!(
            self.owner().is_none(),
            "The shared include already has a parent."
        );
        let shared = self.loaded();
        *self.owner.borrow_mut() = Some(owner.downgrade());
        shared.host.attach(owner);
        self.notify_owner_changed();
    }
    fn remove_owner(&self, owner: &ResourceHostRef) {
        if self.owner().as_ref() == Some(owner) {
            *self.owner.borrow_mut() = None;
            self.loaded().host.detach(owner);
            self.notify_owner_changed();
        }
    }
}
impl Drop for RibbonSharedThemeInclude {
    fn drop(&mut self) {
        if let (Some(owner), Some(shared)) = (
            self.owner
                .get_mut()
                .as_ref()
                .and_then(WeakResourceHost::upgrade),
            self.loaded.get_mut().as_ref(),
        ) {
            shared.host.detach(&owner);
        }
    }
}
ferro_markup_type!(class RibbonSharedThemeInclude {
    namespace: "FerroRibbon.Internal",
    this: Rc<RibbonSharedThemeInclude>,
    handles: [RibbonSharedThemeInclude, Rc<RibbonSharedThemeInclude>, Option<Rc<RibbonSharedThemeInclude>>],
    interfaces: [Rc<dyn IResourceProvider>],
    constructors: [
        () => RibbonSharedThemeInclude::new,
    ],
    properties: [
        Source: Option<Uri> { get: RibbonSharedThemeInclude::source, set: RibbonSharedThemeInclude::set_source },
        Loaded: Rc<dyn IResourceDictionary> { try_get: RibbonSharedThemeInclude::try_loaded },
    ],
});
