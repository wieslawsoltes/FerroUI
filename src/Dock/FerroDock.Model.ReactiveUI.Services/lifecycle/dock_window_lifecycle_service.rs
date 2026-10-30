// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::overlays::hosting::{
    HostServiceResolverExt, HostTypeRegistry, IHostOverlayServicesProvider, IHostServiceResolver,
};
use ferrodock_model::core::{IDockWindow, IDockable};
use ferrodock_model::services::{IHostOverlayServices, IWindowLifecycleService};
use ferroui_reactiveui::IScreen;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// Source once-per-window overlay cleanup and layout disposal.
pub struct DockWindowLifecycleService {
    provider: Rc<dyn IHostOverlayServicesProvider>,
    resolver: Option<Rc<dyn IHostServiceResolver>>,
    cleaned: RefCell<HashMap<usize, Weak<dyn IDockWindow>>>,
    types: Rc<HostTypeRegistry>,
}
impl DockWindowLifecycleService {
    /// Creates a lifecycle service, optionally using the supplied resolver before provider lookup.
    pub fn new(
        provider: Rc<dyn IHostOverlayServicesProvider>,
        resolver: Option<Rc<dyn IHostServiceResolver>>,
    ) -> Rc<Self> {
        let types = provider
            .native_types()
            .or_else(|| {
                resolver
                    .as_ref()
                    .and_then(|resolver| resolver.native_types())
            })
            .unwrap_or_default();
        Rc::new(Self {
            provider,
            resolver,
            cleaned: RefCell::new(HashMap::new()),
            types,
        })
    }
    /// Gets the native reference projections used by cleanup.
    pub fn types(&self) -> Rc<HostTypeRegistry> {
        self.types.clone()
    }
    /// Cleans overlays on a closed window, once for this window identity.
    pub fn on_window_closed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.cleanup(window);
    }
    /// Uses the same once-per-window cleanup path after removal.
    pub fn on_window_removed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.cleanup(window);
    }
    fn cleanup(&self, window: Option<Rc<dyn IDockWindow>>) {
        let Some(window) = window else {
            return;
        };
        let Some(layout) = window.layout() else {
            return;
        };
        let Some(screen) = self
            .types
            .dockable::<dyn IScreen>(&(layout as Rc<dyn IDockable>))
        else {
            return;
        };
        let key = Rc::as_ptr(&window) as *const () as usize;
        {
            let mut cleaned = self.cleaned.borrow_mut();
            cleaned.retain(|_, window| window.upgrade().is_some());
            if cleaned.contains_key(&key) {
                return;
            }
            cleaned.insert(key, Rc::downgrade(&window));
        }
        let Some(overlays) = self.resolve_overlays(screen) else {
            return;
        };
        overlays.dialogs().cancel_all();
        overlays.confirmations().cancel_all();
        // Source deliberately rereads Layout after callbacks may replace it.
        if let Some(layout) = window.layout() {
            if let Some(disposable) = self
                .types
                .dockable::<dyn IDisposable>(&(layout as Rc<dyn IDockable>))
            {
                disposable.dispose();
            }
        }
    }
    fn resolve_overlays(&self, screen: Rc<dyn IScreen>) -> Option<Rc<dyn IHostOverlayServices>> {
        if let Some(direct) = self.types.screen::<dyn IHostOverlayServices>(&screen) {
            return Some(direct);
        }
        if let Some(resolver) = &self.resolver {
            if let Some(resolved) = resolver.resolve::<dyn IHostOverlayServices>(screen.clone()) {
                return Some(resolved);
            }
        }
        if let Some(provider) = self.provider.as_cached_provider() {
            return provider.try_get_cached(screen);
        }
        Some(self.provider.get_services(screen))
    }
}
impl IWindowLifecycleService for DockWindowLifecycleService {
    fn on_window_closed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.on_window_closed(window);
    }
    fn on_window_removed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.on_window_removed(window);
    }
}
