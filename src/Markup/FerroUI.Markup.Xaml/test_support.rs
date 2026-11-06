//! Hand-built service providers and platform doubles for the tests of this
//! crate.

use crate::xaml_il::runtime::{
    IFerroXamlIlControlTemplateProvider, IFerroXamlIlEagerParentStackProvider, IFerroXamlIlParentStackProvider,
    IFerroXamlIlXmlNamespaceInfoProvider, XmlNamespaces,
};
use crate::{IProvideValueTarget, IRootObjectProvider, IUriContext};
use ferroui_base::controls::{INameScope, NameScopeRef};
use ferroui_base::metadata::{service, IServiceProvider};
use ferroui_base::platform::{AssetAssembly, AssetStream, IAssetLoader};
use ferroui_base::reactive::IDisposable;
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::utilities::{UriExtensions, UriRef};
use ferroui_base::{BoxedValue, FerroLocator, PropertyValue};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Cursor;
use std::rc::{Rc, Weak};

/// Boxes a value.
pub(crate) fn boxed<T: PropertyValue>(value: T) -> BoxedValue {
    Rc::new(value)
}

/// The reference of `text`, as markup reads it (the source of an include).
pub(crate) fn uri_ref(text: &str) -> UriRef {
    UriRef::parse(text).unwrap()
}

/// The absolute URL of `text`, as the loader reads the URI of a document.
pub(crate) fn url(text: &str) -> url::Url {
    UriRef::parse_absolute(text).unwrap()
}

/// A parent stack provider that only enumerates.
struct LazyParents(Rc<Vec<BoxedValue>>);

impl IFerroXamlIlParentStackProvider for LazyParents {
    fn parents(&self) -> Vec<BoxedValue> {
        self.0.iter().rev().cloned().collect()
    }
}

/// A service provider with every service of the runtime context of a
/// document, each of which can be left out.
pub(crate) struct TestServiceProvider {
    this: Weak<TestServiceProvider>,
    /// The parents, the immediate parent last.
    parents: RefCell<Option<Rc<Vec<BoxedValue>>>>,
    eager: Cell<bool>,
    outer: RefCell<Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>>>,
    target: RefCell<Option<(Option<BoxedValue>, Option<BoxedValue>)>>,
    root: RefCell<Option<Option<BoxedValue>>>,
    base_uri: RefCell<Option<Option<url::Url>>>,
    name_scope: RefCell<Option<NameScopeRef>>,
    in_control_template: Cell<bool>,
    namespaces: RefCell<Option<XmlNamespaces>>,
}

impl TestServiceProvider {
    pub(crate) fn new() -> Rc<Self> {
        crate::register_types();
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            parents: RefCell::new(None),
            eager: Cell::new(true),
            outer: RefCell::new(None),
            target: RefCell::new(None),
            root: RefCell::new(None),
            base_uri: RefCell::new(None),
            name_scope: RefCell::new(None),
            in_control_template: Cell::new(false),
            namespaces: RefCell::new(None),
        })
    }

    /// Sets the parents, the outermost first.
    pub(crate) fn with_parents(self: Rc<Self>, parents: Vec<BoxedValue>) -> Rc<Self> {
        *self.parents.borrow_mut() = Some(Rc::new(parents));
        self
    }

    /// Makes the parent stack provider one that only enumerates.
    pub(crate) fn lazy(self: Rc<Self>) -> Rc<Self> {
        self.eager.set(false);
        self
    }

    pub(crate) fn with_outer_parents(self: Rc<Self>, outer: Rc<TestServiceProvider>) -> Rc<Self> {
        *self.outer.borrow_mut() = Some(outer);
        self
    }

    pub(crate) fn with_target(self: Rc<Self>, object: Option<BoxedValue>, property: Option<BoxedValue>) -> Rc<Self> {
        *self.target.borrow_mut() = Some((object, property));
        self
    }

    pub(crate) fn with_root(self: Rc<Self>, root: Option<BoxedValue>) -> Rc<Self> {
        *self.root.borrow_mut() = Some(root);
        self
    }

    pub(crate) fn with_base_uri(self: Rc<Self>, base_uri: &str) -> Rc<Self> {
        *self.base_uri.borrow_mut() = Some(Some(url(base_uri)));
        self
    }

    pub(crate) fn with_name_scope(self: Rc<Self>, scope: NameScopeRef) -> Rc<Self> {
        *self.name_scope.borrow_mut() = Some(scope);
        self
    }

    pub(crate) fn in_control_template(self: Rc<Self>) -> Rc<Self> {
        self.in_control_template.set(true);
        self
    }

    pub(crate) fn with_namespaces(self: Rc<Self>, namespaces: XmlNamespaces) -> Rc<Self> {
        *self.namespaces.borrow_mut() = Some(namespaces);
        self
    }

    /// The provider as the service provider contract.
    pub(crate) fn sp(self: &Rc<Self>) -> Rc<dyn IServiceProvider> {
        self.clone()
    }

    fn this(&self) -> Rc<Self> {
        self.this.upgrade().unwrap()
    }
}

impl IServiceProvider for TestServiceProvider {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        if service_type == TypeId::of::<Rc<dyn IFerroXamlIlParentStackProvider>>() {
            let parents = self.parents.borrow().clone()?;
            let provider: Rc<dyn IFerroXamlIlParentStackProvider> =
                if self.eager.get() { self.this() } else { Rc::new(LazyParents(parents)) };
            return Some(Rc::new(provider));
        }
        if service_type == TypeId::of::<Rc<dyn IProvideValueTarget>>() {
            self.target.borrow().as_ref()?;
            return service(service_type, || -> Rc<dyn IProvideValueTarget> { self.this() });
        }
        if service_type == TypeId::of::<Rc<dyn IRootObjectProvider>>() {
            self.root.borrow().as_ref()?;
            return service(service_type, || -> Rc<dyn IRootObjectProvider> { self.this() });
        }
        if service_type == TypeId::of::<Rc<dyn IUriContext>>() {
            self.base_uri.borrow().as_ref()?;
            return service(service_type, || -> Rc<dyn IUriContext> { self.this() });
        }
        if service_type == TypeId::of::<Rc<dyn INameScope>>() {
            let scope = self.name_scope.borrow().clone()?;
            return service(service_type, || -> Rc<dyn INameScope> { scope.0 });
        }
        if service_type == TypeId::of::<Rc<dyn IFerroXamlIlControlTemplateProvider>>() {
            if !self.in_control_template.get() {
                return None;
            }
            return service(service_type, || -> Rc<dyn IFerroXamlIlControlTemplateProvider> { self.this() });
        }
        if service_type == TypeId::of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>() {
            self.namespaces.borrow().as_ref()?;
            return service(service_type, || -> Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider> { self.this() });
        }
        None
    }
}

impl IFerroXamlIlParentStackProvider for TestServiceProvider {
    fn parents(&self) -> Vec<BoxedValue> {
        let mut parents: Vec<BoxedValue> =
            self.parents.borrow().as_ref().map(|p| p.iter().rev().cloned().collect()).unwrap_or_default();
        if let Some(outer) = &*self.outer.borrow() {
            parents.extend(outer.parents());
        }
        parents
    }

    fn as_eager_parent_stack_provider(self: Rc<Self>) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        Some(self)
    }
}

impl IFerroXamlIlEagerParentStackProvider for TestServiceProvider {
    fn direct_parents_stack(&self) -> Rc<Vec<BoxedValue>> {
        self.parents.borrow().clone().unwrap_or_default()
    }

    fn parent_provider(&self) -> Option<Rc<dyn IFerroXamlIlEagerParentStackProvider>> {
        self.outer.borrow().clone()
    }
}

impl IProvideValueTarget for TestServiceProvider {
    fn target_object(&self) -> Option<BoxedValue> {
        self.target.borrow().as_ref().and_then(|t| t.0.clone())
    }

    fn target_property(&self) -> Option<BoxedValue> {
        self.target.borrow().as_ref().and_then(|t| t.1.clone())
    }
}

impl IRootObjectProvider for TestServiceProvider {
    fn root_object(&self) -> Option<BoxedValue> {
        self.root.borrow().clone().flatten()
    }

    fn intermediate_root_object(&self) -> Option<BoxedValue> {
        self.root_object()
    }
}

impl IUriContext for TestServiceProvider {
    fn base_uri(&self) -> Option<url::Url> {
        self.base_uri.borrow().clone().flatten()
    }

    fn set_base_uri(&self, value: Option<url::Url>) {
        *self.base_uri.borrow_mut() = Some(value);
    }
}

impl IFerroXamlIlControlTemplateProvider for TestServiceProvider {}

impl IFerroXamlIlXmlNamespaceInfoProvider for TestServiceProvider {
    fn xml_namespaces(&self) -> XmlNamespaces {
        self.namespaces.borrow().clone().unwrap()
    }
}

/// An asset loader over documents held in memory. The assembly of a URI is
/// its authority: the registered markup assembly of that name, ignoring case
/// (the authority of an absolute URI is in lower case, as the standard asset
/// loader reads it), or the authority itself.
pub(crate) struct TestAssetLoader {
    assets: RefCell<HashMap<String, Vec<u8>>>,
}

impl TestAssetLoader {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self { assets: RefCell::new(HashMap::new()) })
    }

    pub(crate) fn add(&self, absolute_uri: &str, content: &str) {
        self.assets.borrow_mut().insert(url(absolute_uri).as_str().to_owned(), content.as_bytes().to_vec());
    }

    fn absolute(uri: &UriRef, base_uri: Option<&url::Url>) -> Option<url::Url> {
        UriExtensions::ensure_absolute(uri, base_uri).ok()
    }

    fn assembly_of(uri: &url::Url) -> AssetAssembly {
        let authority = UriExtensions::authority(uri);
        let registered = MarkupAssembly::registered_assemblies()
            .into_iter()
            .find(|assembly| assembly.name.eq_ignore_ascii_case(authority));
        AssetAssembly::new(registered.map_or(authority, |assembly| assembly.name))
    }

    /// Makes the loader the asset loader of the calling thread until the
    /// returned scope is disposed.
    pub(crate) fn install(self: &Rc<Self>) -> Rc<dyn IDisposable> {
        let scope = FerroLocator::enter_scope();
        let loader: Rc<dyn IAssetLoader> = self.clone();
        FerroLocator::current_mutable().bind::<dyn IAssetLoader>().to_constant(loader);
        scope
    }
}

impl IAssetLoader for TestAssetLoader {
    fn set_default_assembly(&self, _assembly: &AssetAssembly) {}

    fn exists(&self, uri: &UriRef, base_uri: Option<&url::Url>) -> bool {
        Self::absolute(uri, base_uri).is_some_and(|uri| self.assets.borrow().contains_key(uri.as_str()))
    }

    fn open(&self, uri: &UriRef, base_uri: Option<&url::Url>) -> std::io::Result<Box<dyn AssetStream>> {
        self.open_and_get_assembly(uri, base_uri).map(|(stream, _)| stream)
    }

    fn open_and_get_assembly(
        &self,
        uri: &UriRef,
        base_uri: Option<&url::Url>,
    ) -> std::io::Result<(Box<dyn AssetStream>, AssetAssembly)> {
        let not_found = || std::io::Error::new(std::io::ErrorKind::NotFound, format!("The resource {uri} could not be found."));
        let absolute = Self::absolute(uri, base_uri).ok_or_else(not_found)?;
        let content = self.assets.borrow().get(absolute.as_str()).cloned().ok_or_else(not_found)?;
        let assembly = Self::assembly_of(&absolute);
        Ok((Box::new(Cursor::new(content)), assembly))
    }

    fn get_assembly(&self, uri: &UriRef, base_uri: Option<&url::Url>) -> Option<AssetAssembly> {
        Self::absolute(uri, base_uri).map(|uri| Self::assembly_of(&uri))
    }

    fn get_assets(&self, _uri: &UriRef, _base_uri: Option<&url::Url>) -> Vec<url::Url> {
        Vec::new()
    }

    fn invalidate_assembly_cache(&self, _name: &str) {}

    fn invalidate_assembly_cache_all(&self) {}
}
