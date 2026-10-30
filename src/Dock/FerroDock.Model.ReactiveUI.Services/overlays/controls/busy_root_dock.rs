// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::navigation::controls::RoutableRootDock;
use crate::navigation::model_contracts::root_contracts;
use crate::overlays::hosting::{HostOverlayServicesFactory, HostTypeRegistry};
use crate::threading::task_action::spawn_catching;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::services::*;
use ferrodock_model::BoxedValue;
use ferrodock_model_reactiveui::core::DockableBaseImpl;
use ferrodock_model_reactiveui::impl_dock;
use ferroui_reactiveui::{reactive_object, ICommand, IRoutableViewModel, IScreen, RoutingState};
use ferroui_system::{threading::tasks::Task, IDisposable};
use std::any::TypeId;
use std::cell::{Cell, OnceCell};
use std::rc::{Rc, Weak};
/// Source routable root exposing per-host overlays and current-content reload.
pub struct BusyRootDock {
    base: RoutableRootDock,
    services: OnceCell<Rc<dyn IHostOverlayServices>>,
    disposed: Cell<bool>,
    types: Rc<HostTypeRegistry>,
    runtime: Weak<Self>,
}
reactive_object!(BusyRootDock, base);
impl DockableBaseImpl for BusyRootDock {}
impl_dock!(@base BusyRootDock,RoutableRootDock,
    fn as_root_dock(self:Rc<Self>)->Option<Rc<dyn IRootDock>> { Some(self) },
    fn boxed_interface(self:Rc<Self>,target:TypeId)->Option<BoxedValue> {
        if target==TypeId::of::<dyn IScreen>() { return Some(Rc::new(self as Rc<dyn IScreen>)); }
        if target==TypeId::of::<dyn IRoutableViewModel>() { return Some(Rc::new(self as Rc<dyn IRoutableViewModel>)); }
        if target==TypeId::of::<dyn IHostOverlayServices>() { return Some(Rc::new(self as Rc<dyn IHostOverlayServices>)); }
        if target==TypeId::of::<dyn IDisposable>() { return Some(Rc::new(self as Rc<dyn IDisposable>)); } None
    }
);
root_contracts!(BusyRootDock);
impl BusyRootDock {
    /// Creates the source root, invokes the host service factory once and installs reload.
    pub fn new(
        host: Rc<dyn IScreen>,
        factory: HostOverlayServicesFactory,
        url: Option<String>,
    ) -> Rc<Self> {
        Self::with_types(host, factory, url, HostTypeRegistry::new())
    }
    /// Uses projections shared by the application's host/navigation composition.
    pub fn with_types(
        host: Rc<dyn IScreen>,
        factory: HostOverlayServicesFactory,
        url: Option<String>,
        types: Rc<HostTypeRegistry>,
    ) -> Rc<Self> {
        let root = Rc::new_cyclic(|runtime| Self {
            base: RoutableRootDock::construct(runtime, host, url),
            services: OnceCell::new(),
            disposed: Cell::new(false),
            types,
            runtime: runtime.clone(),
        });
        root.base.initialize(&root);
        assert!(
            root.services.set(factory()).is_ok(),
            "host services initialized once"
        );
        let weak = root.runtime.clone();
        root.busy().set_reload_handler(Some(Rc::new(move || {
            weak.upgrade()
                .map_or_else(|| Task::from_result(()), |root| root.reload_current_async())
        })));
        root.register_types(&root.types);
        root
    }
    fn services(&self) -> &Rc<dyn IHostOverlayServices> {
        self.services.get().expect("initialized host services")
    }
    /// Gets the same nested router owned by the source routable root.
    pub fn router(&self) -> Rc<RoutingState> {
        self.base.router()
    }
    /// Gets this root's immutable route segment.
    pub fn url_path_segment(&self) -> String {
        self.base.url_path_segment()
    }
    /// Gets the source host while it remains alive.
    pub fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.base.host_screen()
    }
    /// Gets the native type projections used by reload lookup.
    pub fn types(&self) -> Rc<HostTypeRegistry> {
        self.types.clone()
    }
    /// Registers screen/routing/root/service/disposal projections of this same object.
    pub fn register_types(self: &Rc<Self>, types: &HostTypeRegistry) {
        types.register_dockable_screen(self);
        types.register_routable(self);
        types.register_interface(self, self.clone() as Rc<dyn IHostOverlayServices>);
        types.register_interface(self, self.clone() as Rc<dyn IDisposable>);
    }
    /// Clears reload first, then dialogs then confirmations, once. The disposed
    /// marker is set before callbacks; failures do not reset it or trigger retries.
    pub fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.busy().set_reload_handler(None);
        self.dialogs().cancel_all();
        self.confirmations().cancel_all();
    }
    fn reload_current_async(&self) -> Task<()> {
        let root = self.runtime.upgrade().expect("live busy root");
        spawn_catching(async move {
            if let Some(reloadable) = root.find_reloadable(root.active_dockable()) {
                reloadable.reload_async().await?;
            }
            Ok(())
        })
    }
    fn find_reloadable(&self, dockable: Option<Rc<dyn IDockable>>) -> Option<Rc<dyn IReloadable>> {
        let dockable = dockable?;
        if let Some(dock) = dockable.clone().as_dock() {
            if dock.active_dockable().is_some() {
                return self.find_reloadable(dock.active_dockable());
            }
        }
        if let Some(screen) = self.types.dockable::<dyn IScreen>(&dockable) {
            let stack = screen.router().navigation_stack();
            let count = stack.count();
            if count > 0 {
                if let Some(value) = self
                    .types
                    .routed_value(&stack.get(count - 1), TypeId::of::<dyn IReloadable>())
                {
                    return value.get();
                }
            }
        }
        self.types.dockable::<dyn IReloadable>(&dockable)
    }
}
impl IScreen for BusyRootDock {
    fn router(&self) -> Rc<RoutingState> {
        self.router()
    }
}
impl IRoutableViewModel for BusyRootDock {
    fn url_path_segment(&self) -> Option<String> {
        Some(self.url_path_segment())
    }
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host_screen()
    }
}
impl IDisposable for BusyRootDock {
    fn dispose(&self) {
        self.dispose();
    }
}
impl IHostOverlayServices for BusyRootDock {
    fn busy(&self) -> Rc<dyn IDockBusyService> {
        self.services().busy()
    }
    fn dialogs(&self) -> Rc<dyn IDockDialogService> {
        self.services().dialogs()
    }
    fn confirmations(&self) -> Rc<dyn IDockConfirmationService> {
        self.services().confirmations()
    }
    fn global_busy_service(&self) -> Rc<dyn IDockGlobalBusyService> {
        self.services().global_busy_service()
    }
    fn global_dialog_service(&self) -> Rc<dyn IDockGlobalDialogService> {
        self.services().global_dialog_service()
    }
    fn global_confirmation_service(&self) -> Rc<dyn IDockGlobalConfirmationService> {
        self.services().global_confirmation_service()
    }
}

#[cfg(test)]
#[path = "busy_root_dock_tests.rs"]
mod tests;
