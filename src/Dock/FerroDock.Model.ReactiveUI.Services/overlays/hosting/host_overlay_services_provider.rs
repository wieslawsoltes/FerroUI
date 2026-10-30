// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::host_type_registry::identity;
use super::{
    HostServiceResolverExt, HostTypeRegistry, IHostOverlayServicesProvider, IHostServiceResolver,
    OwnerChainHostServiceResolver,
};
use ferrodock_model::core::IDockable;
use ferrodock_model::services::IHostOverlayServices;
use ferroui_reactiveui::IScreen;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// Source fallback factory; invoked only when no resolved/cached services exist.
pub type HostOverlayServicesFactory = Rc<HostOverlayServicesFactoryFn>;
/// The source Func service contract as a Splat generic service key. Resolution
/// returns its Rc handle, HostOverlayServicesFactory, without an extra Rc wrapper.
pub type HostOverlayServicesFactoryFn = dyn Fn() -> Rc<dyn IHostOverlayServices>;
type FallbackEntry = (Weak<dyn IScreen>, Rc<dyn IHostOverlayServices>);
/// Source overlay provider with per-screen weak-key fallback caching.
/// Native pruning handles expired noncyclic keys; CLR ephemeron cycles remain open.
pub struct HostOverlayServicesProvider {
    resolver: Rc<dyn IHostServiceResolver>,
    fallback: HostOverlayServicesFactory,
    cache: RefCell<HashMap<usize, FallbackEntry>>,
    types: Rc<HostTypeRegistry>,
}
impl HostOverlayServicesProvider {
    /// Creates a provider using the supplied fallback factory and resolver projections.
    pub fn new(
        resolver: Rc<dyn IHostServiceResolver>,
        fallback: HostOverlayServicesFactory,
    ) -> Rc<Self> {
        let types = resolver.native_types().unwrap_or_default();
        Rc::new(Self {
            resolver,
            fallback,
            cache: RefCell::new(HashMap::new()),
            types,
        })
    }
    /// Source constructor overload that returns the same fallback instance for every screen.
    pub fn with_fallback(
        resolver: Rc<dyn IHostServiceResolver>,
        fallback: Rc<dyn IHostOverlayServices>,
    ) -> Rc<Self> {
        Self::new(resolver, Rc::new(move || fallback.clone()))
    }
    /// Gets this composition's native reference projections.
    pub fn types(&self) -> Rc<HostTypeRegistry> {
        self.types.clone()
    }
    fn prune(&self) {
        let expired: Vec<_> = self
            .cache
            .borrow()
            .iter()
            .filter_map(|(&key, (screen, _))| screen.upgrade().is_none().then_some(key))
            .collect();
        let removed: Vec<_> = {
            let mut cache = self.cache.borrow_mut();
            expired
                .into_iter()
                .filter_map(|key| cache.remove(&key))
                .collect()
        };
        drop(removed);
    }
    fn remove_cached(&self, screen: &Rc<dyn IScreen>) {
        let removed = self.cache.borrow_mut().remove(&identity(screen));
        drop(removed);
    }
    /// Gets cached fallback services only, without invoking resolver/factory.
    pub fn try_get_cached(&self, screen: Rc<dyn IScreen>) -> Option<Rc<dyn IHostOverlayServices>> {
        self.prune();
        self.cache
            .borrow()
            .get(&identity(&screen))
            .map(|(_, services)| services.clone())
    }
    /// Resolves before consulting the cache; a successful resolution evicts this screen's fallback.
    pub fn get_services(&self, screen: Rc<dyn IScreen>) -> Rc<dyn IHostOverlayServices> {
        if let Some(resolved) = self
            .resolver
            .resolve::<dyn IHostOverlayServices>(screen.clone())
        {
            self.remove_cached(&screen);
            return resolved;
        }
        if let Some(cached) = self.try_get_cached(screen.clone()) {
            return cached;
        }
        let created = (self.fallback)();
        // CWT.GetValue's callback is outside its mutation lock. A reentrant GetValue
        // can insert first; that existing value wins over this outer callback result.
        if let Some(cached) = self.try_get_cached(screen.clone()) {
            return cached;
        }
        self.cache
            .borrow_mut()
            .insert(identity(&screen), (Rc::downgrade(&screen), created.clone()));
        created
    }
    /// Resolves a screen dockable, its first root, then that root's window layout,
    /// before falling back to GetServices(screen). Source uses an always-true root predicate.
    pub fn get_services_for_dockable(
        &self,
        screen: Rc<dyn IScreen>,
        dockable: Rc<dyn IDockable>,
    ) -> Rc<dyn IHostOverlayServices> {
        self.resolve_from_dockable(dockable)
            .unwrap_or_else(|| self.get_services(screen))
    }
    fn resolve_from_dockable(
        &self,
        dockable: Rc<dyn IDockable>,
    ) -> Option<Rc<dyn IHostOverlayServices>> {
        if let Some(screen) = self.types.dockable::<dyn IScreen>(&dockable) {
            if let Some(resolved) = self
                .resolver
                .resolve::<dyn IHostOverlayServices>(screen.clone())
            {
                self.remove_cached(&screen);
                return Some(resolved);
            }
        }
        let factory = OwnerChainHostServiceResolver::find_factory(dockable.clone())?;
        let root = factory.find_root(Some(dockable), Some(Rc::new(|_| true)));
        if let Some(root) = root {
            if let Some(services) = self
                .types
                .dockable::<dyn IHostOverlayServices>(&(root.clone() as Rc<dyn IDockable>))
            {
                return Some(services);
            }
            if let Some(layout) = root.window().and_then(|window| window.layout()) {
                return self
                    .types
                    .dockable::<dyn IHostOverlayServices>(&(layout as Rc<dyn IDockable>));
            }
        }
        None
    }
}
impl IHostOverlayServicesProvider for HostOverlayServicesProvider {
    fn get_services(&self, screen: Rc<dyn IScreen>) -> Rc<dyn IHostOverlayServices> {
        self.get_services(screen)
    }
    fn get_services_for_dockable(
        &self,
        screen: Rc<dyn IScreen>,
        dockable: Rc<dyn IDockable>,
    ) -> Rc<dyn IHostOverlayServices> {
        self.get_services_for_dockable(screen, dockable)
    }
    fn as_cached_provider(&self) -> Option<&HostOverlayServicesProvider> {
        Some(self)
    }
    fn native_types(&self) -> Option<Rc<HostTypeRegistry>> {
        Some(self.types.clone())
    }
}
