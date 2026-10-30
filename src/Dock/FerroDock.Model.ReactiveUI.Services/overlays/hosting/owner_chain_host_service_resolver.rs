// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::host_type_registry::identity;
use super::{HostServiceValue, HostTypeRegistry, IHostServiceResolver};
use ferrodock_model::controls::IRootDock;
use ferrodock_model::core::*;
use ferroui_reactiveui::{IRoutableViewModel, IScreen};
use std::any::TypeId;
use std::collections::HashSet;
use std::rc::Rc;

/// Source owner/window/factory/router host resolution with original lookup precedence.
pub struct OwnerChainHostServiceResolver {
    types: Rc<HostTypeRegistry>,
}
impl OwnerChainHostServiceResolver {
    /// Creates a resolver using an empty native projection registry.
    pub fn new() -> Rc<Self> {
        Self::with_types(HostTypeRegistry::new())
    }
    /// Creates a resolver with projections shared by the service composition root.
    pub fn with_types(types: Rc<HostTypeRegistry>) -> Rc<Self> {
        Rc::new(Self { types })
    }
    /// Gets the native reference projections used for source interface membership.
    pub fn types(&self) -> Rc<HostTypeRegistry> {
        self.types.clone()
    }
    fn resolve_from_screen(
        &self,
        screen: &Rc<dyn IScreen>,
        target: TypeId,
    ) -> Option<HostServiceValue> {
        if let Some(dockable) = self.types.screen::<dyn IDockable>(screen) {
            return self
                .find_in_host_windows(dockable.clone(), target)
                .or_else(|| self.find_in_owner_chain_last(dockable.clone(), target))
                .or_else(|| self.find_in_factory(dockable, target));
        }
        self.types.screen_value(screen, target)
    }
    fn resolve_from_router(
        &self,
        screen: &Rc<dyn IScreen>,
        target: TypeId,
    ) -> Option<HostServiceValue> {
        let stack = screen.router().navigation_stack();
        let mut index = stack.count();
        while index > 0 {
            index -= 1;
            if let Some(service) = self.types.routed_value(&stack.get(index), target) {
                return Some(service);
            }
        }
        None
    }
    pub(crate) fn find_factory(dockable: Rc<dyn IDockable>) -> Option<Rc<dyn IFactory>> {
        let mut current = Some(dockable);
        while let Some(dockable) = current {
            if let Some(factory) = dockable.factory() {
                return Some(factory);
            }
            current = dockable.owner();
        }
        None
    }
    // The source retains this private first-match helper, although its current
    // resolution paths use the last-match variant.
    #[allow(dead_code)]
    fn find_in_owner_chain(
        &self,
        dockable: Rc<dyn IDockable>,
        target: TypeId,
    ) -> Option<HostServiceValue> {
        let mut current = Some(dockable);
        while let Some(dockable) = current {
            if let Some(service) = self.types.dockable_value(&dockable, target) {
                return Some(service);
            }
            current = dockable.owner();
        }
        None
    }
    fn find_in_owner_chain_last(
        &self,
        dockable: Rc<dyn IDockable>,
        target: TypeId,
    ) -> Option<HostServiceValue> {
        let mut result = None;
        let mut current = Some(dockable);
        while let Some(dockable) = current {
            if let Some(service) = self.types.dockable_value(&dockable, target) {
                result = Some(service);
            }
            current = dockable.owner();
        }
        result
    }
    fn find_in_host_windows(
        &self,
        dockable: Rc<dyn IDockable>,
        target: TypeId,
    ) -> Option<HostServiceValue> {
        let factory = Self::find_factory(dockable.clone())?;
        for host in factory.host_windows().iter() {
            let Some(window) = host.window() else {
                continue;
            };
            let Some(layout) = window.layout() else {
                continue;
            };
            let layout: Rc<dyn IDock> = layout;
            if identity(&layout) == identity(&dockable)
                || factory
                    .find_dockable(
                        Some(layout),
                        Some(Rc::new({
                            let dockable = dockable.clone();
                            move |item| {
                                item.as_ref()
                                    .is_some_and(|item| identity(item) == identity(&dockable))
                            }
                        })),
                    )
                    .is_some()
            {
                if let Some(layout) = window.layout() {
                    if let Some(service) = self
                        .types
                        .dockable_value(&(layout as Rc<dyn IDockable>), target)
                    {
                        return Some(service);
                    }
                }
            }
        }
        None
    }
    fn find_in_factory(
        &self,
        dockable: Rc<dyn IDockable>,
        target: TypeId,
    ) -> Option<HostServiceValue> {
        let factory = Self::find_factory(dockable.clone())?;
        for item in factory.find(Some(Rc::new(|item| {
            item.and_then(|item| item.as_root_dock()).is_some()
        }))) {
            let Some(root) = item.and_then(|item| item.as_root_dock()) else {
                continue;
            };
            if identity(&root) == identity(&dockable)
                || factory
                    .find_in_dock(
                        Some(root.clone()),
                        Some(Rc::new({
                            let dockable = dockable.clone();
                            move |item| {
                                item.as_ref()
                                    .is_some_and(|item| identity(item) == identity(&dockable))
                            }
                        })),
                    )
                    .into_iter()
                    .next()
                    .is_some()
            {
                if let Some(service) = self
                    .types
                    .dockable_value(&(root as Rc<dyn IDockable>), target)
                {
                    return Some(service);
                }
            }
        }
        // Source deliberately obtains a second fresh Find sequence for floating windows.
        for item in factory.find(Some(Rc::new(|item| {
            item.and_then(|item| item.as_root_dock()).is_some()
        }))) {
            let Some(root) = item.and_then(|item| item.as_root_dock()) else {
                continue;
            };
            let Some(windows) = root.windows() else {
                continue;
            };
            for window in windows.iter() {
                let window = window.expect("source window.Layout access");
                let Some(layout) = window.layout() else {
                    continue;
                };
                let layout: Rc<dyn IDock> = layout;
                if identity(&layout) == identity(&dockable)
                    || factory
                        .find_dockable(
                            Some(layout.clone()),
                            Some(Rc::new({
                                let dockable = dockable.clone();
                                move |item| {
                                    item.as_ref()
                                        .is_some_and(|item| identity(item) == identity(&dockable))
                                }
                            })),
                        )
                        .is_some()
                {
                    if let Some(service) = self
                        .types
                        .dockable_value(&(layout as Rc<dyn IDockable>), target)
                    {
                        return Some(service);
                    }
                }
            }
        }
        None
    }
    fn normalize_root(&self, service: HostServiceValue, target: TypeId) -> HostServiceValue {
        let Some(root) = self.types.query_value::<dyn IRootDock>(&service) else {
            return service;
        };
        if root.window().is_some() {
            return service;
        }
        let Some(routable) = self.types.query_value::<dyn IRoutableViewModel>(&service) else {
            return service;
        };
        let Some(host) = routable.host_screen() else {
            return service;
        };
        let Some(host) = self.types.screen::<dyn IDockable>(&host) else {
            return service;
        };
        let outer = self
            .find_in_owner_chain_last(host.clone(), target)
            .or_else(|| self.find_in_host_windows(host.clone(), target))
            .or_else(|| self.find_in_factory(host, target));
        if let Some(outer) = outer {
            if let Some(outer_root) = self.types.query_value::<dyn IRootDock>(&outer) {
                if identity(&outer_root) != identity(&root) {
                    return outer;
                }
            }
        }
        service
    }
}
impl IHostServiceResolver for OwnerChainHostServiceResolver {
    fn resolve_erased(&self, screen: Rc<dyn IScreen>, target: TypeId) -> Option<HostServiceValue> {
        let mut visited = HashSet::new();
        let mut current = Some(screen);
        while let Some(screen) = current {
            if !visited.insert(identity(&screen)) {
                break;
            }
            if let Some(service) = self.resolve_from_screen(&screen, target) {
                return Some(self.normalize_root(service, target));
            }
            if let Some(service) = self.resolve_from_router(&screen, target) {
                return Some(self.normalize_root(service, target));
            }
            if let Some(routable) = self.types.screen::<dyn IRoutableViewModel>(&screen) {
                if let Some(next) = routable.host_screen() {
                    current = Some(next);
                    continue;
                }
            }
            break;
        }
        None
    }
    fn native_types(&self) -> Option<Rc<HostTypeRegistry>> {
        Some(self.types.clone())
    }
}
