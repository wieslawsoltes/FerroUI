// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::IDockNavigationService;
use crate::overlays::hosting::HostTypeRegistry;
use ferrodock_model::controls::IDocumentDock;
use ferrodock_model::core::*;
use ferroui_reactiveui::IScreen;
use std::cell::RefCell;
use std::rc::Rc;
/// Source navigation service; all mutations go through the resolved actual factory.
pub struct DockNavigationService {
    factory: RefCell<Option<Rc<dyn IFactory>>>,
    host: RefCell<Option<Rc<dyn IScreen>>>,
    types: Rc<HostTypeRegistry>,
}
impl DockNavigationService {
    /// Creates a service with an empty native screen projection registry.
    pub fn new() -> Rc<Self> {
        Self::with_types(HostTypeRegistry::new())
    }
    /// Uses native interface projections shared by the application's composition root.
    pub fn with_types(types: Rc<HostTypeRegistry>) -> Rc<Self> {
        Rc::new(Self {
            factory: RefCell::new(None),
            host: RefCell::new(None),
            types,
        })
    }
    /// Gets this service's native interface projections.
    pub fn types(&self) -> Rc<HostTypeRegistry> {
        self.types.clone()
    }
    /// Retains the fallback factory and host supplied by the source AttachFactory call.
    pub fn attach_factory(&self, factory: Rc<dyn IFactory>, host: Rc<dyn IScreen>) {
        self.factory.replace(Some(factory));
        self.host.replace(Some(host));
    }
    /// Reactivates an existing visible owner member, or adds a missing/new document.
    /// IDs are not used for deduplication. Existing documents activate their window
    /// before optional Float; newly added documents do not explicitly activate it.
    pub fn open_document(
        &self,
        host_screen: Rc<dyn IScreen>,
        document: Rc<dyn IDockable>,
        float_window: bool,
    ) {
        let Some((factory, dock)) = self.try_get_document_context(Some(host_screen)) else {
            return;
        };
        if let Some(owner) = document.owner().and_then(|owner| owner.as_dock()) {
            if owner
                .visible_dockables()
                .is_some_and(|items| items.contains(&Some((document).clone())))
            {
                factory.set_active_dockable(Some(document.clone()));
                factory.set_focused_dockable(Some(owner), Some(document.clone()));
                factory.activate_window(Some(document.clone()));
                if float_window {
                    factory.float_dockable(Some(document));
                }
                return;
            }
        }
        factory.add_dockable(Some(dock.clone()), Some(document.clone()));
        factory.set_active_dockable(Some(document.clone()));
        factory.set_focused_dockable(Some(dock), Some(document.clone()));
        if float_window {
            factory.float_dockable(Some(document));
        }
    }
    /// Source protected helper exposed for native composition: tries the supplied
    /// host first, then a distinct attached host only if the first context failed.
    #[doc(hidden)]
    pub fn try_get_document_context(
        &self,
        host: Option<Rc<dyn IScreen>>,
    ) -> Option<(Rc<dyn IFactory>, Rc<dyn IDocumentDock>)> {
        if let Some(context) = self.try_resolve_document_dock(host.clone()) {
            return Some(context);
        }
        let fallback = self.host.borrow().clone()?;
        if host
            .as_ref()
            .is_some_and(|host| std::ptr::addr_eq(Rc::as_ptr(host), Rc::as_ptr(&fallback)))
        {
            return None;
        }
        self.try_resolve_document_dock(Some(fallback))
    }
    /// Source protected owner-chain lookup, preserving independent first factory/
    /// document-dock discovery and direct-child/default root fallback.
    #[doc(hidden)]
    pub fn try_resolve_document_dock(
        &self,
        host: Option<Rc<dyn IScreen>>,
    ) -> Option<(Rc<dyn IFactory>, Rc<dyn IDocumentDock>)> {
        let dockable = self.types.screen::<dyn IDockable>(&host?)?;
        let mut current = Some(dockable.clone());
        let mut factory = None;
        let mut dock = None;
        while let Some(item) = current {
            if factory.is_none() {
                factory = item.factory();
            }
            if dock.is_none() {
                dock = item.clone().as_document_dock();
            }
            if factory.is_some() && dock.is_some() {
                break;
            }
            current = item.owner();
        }
        let factory = factory.or_else(|| self.factory.borrow().clone())?;
        if dock.is_none() {
            if let Some(root) = factory.find_root(Some(dockable), None) {
                dock = root
                    .default_dockable()
                    .and_then(|item| item.as_document_dock());
                if dock.is_none() {
                    if let Some(items) = root.visible_dockables() {
                        for item in items.iter() {
                            if let Some(found) = item.and_then(|item| item.as_document_dock()) {
                                dock = Some(found);
                                break;
                            }
                        }
                    }
                }
            }
        }
        Some((factory, dock?))
    }
}
impl IDockNavigationService for DockNavigationService {
    fn attach_factory(&self, factory: Rc<dyn IFactory>, host: Rc<dyn IScreen>) {
        self.attach_factory(factory, host);
    }
    fn open_document(
        &self,
        host: Rc<dyn IScreen>,
        document: Rc<dyn IDockable>,
        float_window: bool,
    ) {
        self.open_document(host, document, float_window);
    }
}
