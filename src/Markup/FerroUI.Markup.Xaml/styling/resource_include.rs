//! Port of `Styling/ResourceInclude.cs`.

use crate::{FerroXamlLoader, FromXamlObject, ServiceProviderExtensions, XamlLoadError};
use ferroui_base::controls::{
    IResourceDictionary, IResourceNode, IResourceProvider, IThemeVariantProvider, ResourceHostRef, ResourceKey,
    ResourceValue,
};
use ferroui_base::metadata::{IServiceProvider, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::UriRef;
use ferroui_base::ferro_markup_type;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Loads a resource dictionary from a specified URL.
///
/// The document is loaded the first time the resources are asked for.
pub struct ResourceInclude {
    this: Weak<ResourceInclude>,
    service_provider: Option<Rc<dyn IServiceProvider>>,
    base_uri: Option<url::Url>,
    loaded: RefCell<Option<Rc<dyn IResourceDictionary>>>,
    is_loading: Cell<bool>,
    source: RefCell<Option<UriRef>>,
    key: RefCell<Option<ThemeVariant>>,
    #[cfg(feature = "source-overlays")]
    source_snapshot: Option<crate::source_overlay::SourceSnapshot>,
}

impl ResourceInclude {
    /// Initializes a new instance: `base_uri` is the URL of the document
    /// that contains the include, which a relative source is resolved
    /// against ([`UriRef::resolve`]).
    pub fn new(base_uri: Option<url::Url>) -> Rc<Self> {
        Self::create(None, base_uri)
    }

    /// Initializes a new instance with the service provider of the place
    /// the include is declared in.
    pub fn with_service_provider(service_provider: Rc<dyn IServiceProvider>) -> Rc<Self> {
        let base_uri = service_provider.get_context_base_uri();
        Self::create(Some(service_provider), base_uri)
    }

    fn create(service_provider: Option<Rc<dyn IServiceProvider>>, base_uri: Option<url::Url>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            service_provider,
            base_uri,
            loaded: RefCell::new(None),
            is_loading: Cell::new(false),
            source: RefCell::new(None),
            key: RefCell::new(None),
            #[cfg(feature = "source-overlays")]
            source_snapshot: crate::source_overlay::capture_snapshot(),
        })
    }

    /// The loaded resource dictionary; loads it on first use.
    ///
    /// # Panics
    /// Panics if [`source`](Self::source) is not set, if the document cannot
    /// be loaded or if it is not a resource dictionary. Use
    /// [`try_loaded`](Self::try_loaded) to handle a document that cannot be
    /// loaded.
    pub fn loaded(&self) -> Rc<dyn IResourceDictionary> {
        crate::throw(self.try_loaded())
    }

    /// The loaded resource dictionary for the members of the resource contracts, which have
    /// no error channel: inside an error scope of a load
    /// ([`with_xaml_errors`](crate::xaml_error_scope::with_xaml_errors)), a
    /// document that cannot be loaded is the error of the scope, and the
    /// include acts as if it were empty (#567); outside, it panics as
    /// [`loaded`](Self::loaded) does.
    fn loaded_or_reported(&self) -> Option<Rc<dyn IResourceDictionary>> {
        crate::throw_or(self.try_loaded().map(Some), || None)
    }

    /// The loaded resource dictionary; loads it on first use. A missing
    /// [`source`](Self::source), a document that cannot be loaded and a
    /// document that is not a resource dictionary are errors.
    pub fn try_loaded(&self) -> Result<Rc<dyn IResourceDictionary>, XamlLoadError> {
        if let Some(loaded) = &*self.loaded.borrow() {
            return Ok(loaded.clone());
        }

        self.is_loading.set(true);
        struct LoadingGuard<'a>(&'a Cell<bool>);
        impl Drop for LoadingGuard<'_> {
            fn drop(&mut self) { self.0.set(false); }
        }
        let _loading = LoadingGuard(&self.is_loading);
        let Some(source) = self.source() else {
            return Err(XamlLoadError::with_message("ResourceInclude.Source must be set."));
        };
        let load = ||
            FerroXamlLoader::load_with_service_provider(self.service_provider.as_ref(), &source, self.base_uri.as_ref());
        #[cfg(feature = "source-overlays")]
        let loaded = if let Some(snapshot) = &self.source_snapshot { snapshot.run(load) } else { load() }?;
        #[cfg(not(feature = "source-overlays"))]
        let loaded = load()?;
        let Some(loaded) = <Rc<dyn IResourceDictionary>>::from_xaml_object(&loaded) else {
            return Err(XamlLoadError::with_message(format!(
                "Unable to cast object of type '{}' to type 'IResourceDictionary'.",
                (*loaded).type_name()
            )));
        };
        *self.loaded.borrow_mut() = Some(loaded.clone());

        Ok(loaded)
    }

    /// The source URL: an absolute URL, or a reference relative to the base
    /// URI of the include.
    pub fn source(&self) -> Option<UriRef> {
        self.source.borrow().clone()
    }

    pub fn set_source(&self, value: Option<UriRef>) {
        #[cfg(feature = "source-overlays")]
        if let (Some(snapshot), Some(source)) = (&self.source_snapshot, &value) {
            let absolute = match &self.base_uri {
                Some(base) => source.resolve(base).ok(),
                None => source.as_url().cloned(),
            };
            if let Some(absolute) = absolute {
                snapshot.record_request(absolute.as_str());
            }
        }
        *self.source.borrow_mut() = value;
    }

    /// The include as the resource provider contract.
    pub fn as_resource_provider(&self) -> Rc<dyn IResourceProvider> {
        self.this.upgrade().expect("the include is alive while it is used")
    }

    /// The include as the theme variant provider contract.
    pub fn as_theme_variant_provider(&self) -> Rc<dyn IThemeVariantProvider> {
        self.this.upgrade().expect("the include is alive while it is used")
    }
}

impl IResourceNode for ResourceInclude {
    fn has_resources(&self) -> bool {
        self.loaded_or_reported().is_some_and(|loaded| loaded.has_resources())
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        if !self.is_loading.get() {
            return self.loaded_or_reported()?.try_get_resource(key, theme);
        }

        None
    }
}

impl IResourceProvider for ResourceInclude {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    fn development_source(&self) -> Option<String> {
        let source = self.source()?;
        let uri = match source.as_url() {
            Some(url) => url.clone(),
            None => source.resolve(self.base_uri.as_ref()?).ok()?,
        };
        Some(uri.into())
    }

    fn owner(&self) -> Option<ResourceHostRef> {
        self.loaded_or_reported()?.owner()
    }

    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        match self.loaded_or_reported() {
            Some(loaded) => loaded.owner_changed(handler),
            None => Disposable::empty(),
        }
    }

    fn add_owner(&self, owner: &ResourceHostRef) {
        if let Some(loaded) = self.loaded_or_reported() {
            loaded.add_owner(owner);
        }
    }

    fn remove_owner(&self, owner: &ResourceHostRef) {
        if let Some(loaded) = self.loaded_or_reported() {
            loaded.remove_owner(owner);
        }
    }
}

impl IThemeVariantProvider for ResourceInclude {
    fn key(&self) -> Option<ThemeVariant> {
        self.key.borrow().clone()
    }

    fn set_key(&self, value: Option<ThemeVariant>) {
        *self.key.borrow_mut() = value;
    }
}

crate::identity_eq!(ResourceInclude);

ferro_markup_type!(class ResourceInclude {
    this: Rc<ResourceInclude>,
    handles: [ResourceInclude, Rc<ResourceInclude>, Option<Rc<ResourceInclude>>],
    interfaces: [Rc<dyn IResourceProvider>, Rc<dyn IThemeVariantProvider>],
    constructors: [
        (Option<url::Url>) => ResourceInclude::new,
        (Rc<dyn IServiceProvider>) => ResourceInclude::with_service_provider,
    ],
    properties: [
        Loaded: Rc<dyn IResourceDictionary> { try_get: ResourceInclude::try_loaded },
        Owner: Option<ResourceHostRef> {
            try_get: |this: &Rc<ResourceInclude>| this.try_loaded().map(|loaded| loaded.owner())
        },
        Source: Option<UriRef> { get: ResourceInclude::source, set: ResourceInclude::set_source },
    ],
    events: [
        // As in the original, subscribing loads the dictionary: a failing
        // load is the error of the subscription.
        try OwnerChanged() => |this: &Rc<ResourceInclude>, handler: MarkupDelegate| {
            this.try_loaded().map(|loaded| {
                loaded.owner_changed(Rc::new(move || {
                    handler.invoke(&[]);
                }))
            })
        },
    ],
});
