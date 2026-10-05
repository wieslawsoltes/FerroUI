//! Port of `Styling/StyleInclude.cs`.

use crate::{FerroXamlLoader, FromXamlObject, ServiceProviderExtensions, XamlLoadException};
use ferroui_base::controls::{IResourceNode, IResourceProvider, ResourceHostRef, ResourceKey, ResourceValue};
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{IServiceProvider, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{IStyle, ThemeVariant};
use ferroui_base::utilities::Uri;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Includes a style from a URL.
///
/// The document is loaded the first time the style is asked for.
pub struct StyleInclude {
    this: Weak<StyleInclude>,
    service_provider: Option<Rc<dyn IServiceProvider>>,
    base_uri: Option<Uri>,
    loaded: RefCell<Option<Rc<Vec<Rc<dyn IStyle>>>>>,
    is_loading: Cell<bool>,
    source: RefCell<Option<Uri>>,
}

thread_local! {
    static NO_STYLES: Rc<Vec<Rc<dyn IStyle>>> = Rc::new(Vec::new());
}

impl StyleInclude {
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
        })
    }

    /// The owner of the loaded style, if it is a resource provider.
    pub fn owner(&self) -> Option<ResourceHostRef> {
        self.loaded().as_resource_provider()?.owner()
    }

    /// The source URL.
    pub fn source(&self) -> Option<Uri> {
        self.source.borrow().clone()
    }

    pub fn set_source(&self, value: Option<Uri>) {
        *self.source.borrow_mut() = value;
    }

    /// The loaded style; loads it on first use.
    ///
    /// # Panics
    /// Panics if [`source`](Self::source) is not set, if the document cannot
    /// be loaded or if it is not a style. Use
    /// [`try_loaded`](Self::try_loaded) to handle a document that cannot be
    /// loaded.
    pub fn loaded(&self) -> Rc<dyn IStyle> {
        crate::throw(self.try_loaded())
    }

    /// The loaded style; loads it on first use. A missing
    /// [`source`](Self::source), a document that cannot be loaded and a
    /// document that is not a style are errors.
    pub fn try_loaded(&self) -> Result<Rc<dyn IStyle>, XamlLoadException> {
        if let Some(loaded) = &*self.loaded.borrow() {
            return Ok(loaded[0].clone());
        }

        self.is_loading.set(true);
        let Some(source) = self.source() else {
            return Err(XamlLoadException::with_message("StyleInclude.Source must be set."));
        };
        let loaded =
            FerroXamlLoader::load_with_service_provider(self.service_provider.as_ref(), &source, self.base_uri.as_ref())?;
        let Some(loaded) = <Rc<dyn IStyle>>::from_xaml_object(&loaded) else {
            return Err(XamlLoadException::with_message(format!(
                "Unable to cast object of type '{}' to type 'IStyle'.",
                (*loaded).type_name()
            )));
        };
        *self.loaded.borrow_mut() = Some(Rc::new(vec![loaded.clone()]));
        self.is_loading.set(false);

        Ok(loaded)
    }

    /// The include as the style contract.
    pub fn as_style(&self) -> Rc<dyn IStyle> {
        self.this.upgrade().expect("the include is alive while it is used")
    }
}

impl IResourceNode for StyleInclude {
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

impl IStyle for StyleInclude {
    fn children(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        match &*self.loaded.borrow() {
            Some(loaded) => loaded.clone(),
            None => NO_STYLES.with(Rc::clone),
        }
    }

    fn as_resource_provider(&self) -> Option<&dyn IResourceProvider> {
        Some(self)
    }
}

impl IResourceProvider for StyleInclude {
    fn owner(&self) -> Option<ResourceHostRef> {
        StyleInclude::owner(self)
    }

    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        match self.loaded().as_resource_provider() {
            Some(rp) => rp.owner_changed(handler),
            None => Disposable::empty(),
        }
    }

    fn add_owner(&self, owner: &ResourceHostRef) {
        if let Some(rp) = self.loaded().as_resource_provider() {
            rp.add_owner(owner);
        }
    }

    fn remove_owner(&self, owner: &ResourceHostRef) {
        if let Some(rp) = self.loaded().as_resource_provider() {
            rp.remove_owner(owner);
        }
    }
}

crate::identity_eq!(StyleInclude);

ferro_markup_type!(class StyleInclude {
    this: Rc<StyleInclude>,
    handles: [StyleInclude, Rc<StyleInclude>, Option<Rc<StyleInclude>>],
    interfaces: [Rc<dyn IStyle>, Rc<dyn IResourceProvider>],
    constructors: [
        (Option<Uri>) => StyleInclude::new,
        (Rc<dyn IServiceProvider>) => StyleInclude::with_service_provider,
    ],
    properties: [
        Owner: Option<ResourceHostRef> {
            try_get: |this: &Rc<StyleInclude>| {
                this.try_loaded().map(|loaded| loaded.as_resource_provider().and_then(|provider| provider.owner()))
            }
        },
        Source: Option<Uri> { get: StyleInclude::source, set: StyleInclude::set_source },
        Loaded: Rc<dyn IStyle> { try_get: StyleInclude::try_loaded },
    ],
    events: [
        // As in the original, subscribing loads the style: a failing load is
        // the error of the subscription.
        try OwnerChanged() => |this: &Rc<StyleInclude>, handler: MarkupDelegate| {
            this.try_loaded().map(|loaded| match loaded.as_resource_provider() {
                Some(rp) => rp.owner_changed(Rc::new(move || {
                    handler.invoke(&[]);
                })),
                None => Disposable::empty(),
            })
        },
    ],
});
