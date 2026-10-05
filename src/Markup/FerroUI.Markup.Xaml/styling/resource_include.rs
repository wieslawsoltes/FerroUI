//! Port of `Styling/ResourceInclude.cs`.

use crate::{FerroXamlLoader, FromXamlObject, ServiceProviderExtensions, XamlLoadException};
use ferroui_base::controls::{
    IResourceDictionary, IResourceNode, IResourceProvider, IThemeVariantProvider, ResourceHostRef, ResourceKey,
    ResourceValue,
};
use ferroui_base::metadata::{IServiceProvider, MarkupDelegate};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::Uri;
use ferroui_base::ferro_markup_type;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Loads a resource dictionary from a specified URL.
///
/// The document is loaded the first time the resources are asked for.
pub struct ResourceInclude {
    this: Weak<ResourceInclude>,
    service_provider: Option<Rc<dyn IServiceProvider>>,
    base_uri: Option<Uri>,
    loaded: RefCell<Option<Rc<dyn IResourceDictionary>>>,
    is_loading: Cell<bool>,
    source: RefCell<Option<Uri>>,
    key: RefCell<Option<ThemeVariant>>,
}

impl ResourceInclude {
    /// Initializes a new instance: `base_uri` is the URI of the document
    /// that contains the include, which a relative source is resolved
    /// against.
    pub fn new(base_uri: Option<Uri>) -> Rc<Self> {
        Self::create(None, base_uri)
    }

    /// Initializes a new instance with the service provider of the place
    /// the include is declared in.
    pub fn with_service_provider(service_provider: Rc<dyn IServiceProvider>) -> Rc<Self> {
        let base_uri = service_provider.get_context_base_uri();
        Self::create(Some(service_provider), base_uri)
    }

    fn create(service_provider: Option<Rc<dyn IServiceProvider>>, base_uri: Option<Uri>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            service_provider,
            base_uri,
            loaded: RefCell::new(None),
            is_loading: Cell::new(false),
            source: RefCell::new(None),
            key: RefCell::new(None),
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

    /// The loaded resource dictionary; loads it on first use. A missing
    /// [`source`](Self::source), a document that cannot be loaded and a
    /// document that is not a resource dictionary are errors.
    pub fn try_loaded(&self) -> Result<Rc<dyn IResourceDictionary>, XamlLoadException> {
        if let Some(loaded) = &*self.loaded.borrow() {
            return Ok(loaded.clone());
        }

        self.is_loading.set(true);
        let Some(source) = self.source() else {
            return Err(XamlLoadException::with_message("ResourceInclude.Source must be set."));
        };
        let loaded =
            FerroXamlLoader::load_with_service_provider(self.service_provider.as_ref(), &source, self.base_uri.as_ref())?;
        let Some(loaded) = <Rc<dyn IResourceDictionary>>::from_xaml_object(&loaded) else {
            return Err(XamlLoadException::with_message(format!(
                "Unable to cast object of type '{}' to type 'IResourceDictionary'.",
                (*loaded).type_name()
            )));
        };
        *self.loaded.borrow_mut() = Some(loaded.clone());
        self.is_loading.set(false);

        Ok(loaded)
    }

    /// The source URL.
    pub fn source(&self) -> Option<Uri> {
        self.source.borrow().clone()
    }

    pub fn set_source(&self, value: Option<Uri>) {
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
        self.loaded().has_resources()
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        if !self.is_loading.get() {
            return self.loaded().try_get_resource(key, theme);
        }

        None
    }
}

impl IResourceProvider for ResourceInclude {
    fn owner(&self) -> Option<ResourceHostRef> {
        self.loaded().owner()
    }

    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.loaded().owner_changed(handler)
    }

    fn add_owner(&self, owner: &ResourceHostRef) {
        self.loaded().add_owner(owner)
    }

    fn remove_owner(&self, owner: &ResourceHostRef) {
        self.loaded().remove_owner(owner)
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
        (Option<Uri>) => ResourceInclude::new,
        (Rc<dyn IServiceProvider>) => ResourceInclude::with_service_provider,
    ],
    properties: [
        Loaded: Rc<dyn IResourceDictionary> { try_get: ResourceInclude::try_loaded },
        Owner: Option<ResourceHostRef> {
            try_get: |this: &Rc<ResourceInclude>| this.try_loaded().map(|loaded| loaded.owner())
        },
        Source: Option<Uri> { get: ResourceInclude::source, set: ResourceInclude::set_source },
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
