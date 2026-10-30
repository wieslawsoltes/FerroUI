// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::overlays::hosting::IHostOverlayServicesProvider;
use ferrodock_model::core::{IDockable, IFactory};
use ferrodock_model::services::*;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IScreen;
use std::rc::Rc;
/// Source static routing, close and overlay lookup helpers.
pub struct DockNavigationHelpers;
impl DockNavigationHelpers {
    /// Forwards the screen directly to the provider.
    pub fn get_overlay_services(
        host: Rc<dyn IScreen>,
        provider: &dyn IHostOverlayServicesProvider,
    ) -> Rc<dyn IHostOverlayServices> {
        provider.get_services(host)
    }
    /// Uses the dockable-context provider overload with the supplied fallback screen.
    pub fn get_overlay_services_for_dockable(
        host: Rc<dyn IScreen>,
        dockable: Rc<dyn IDockable>,
        provider: &dyn IHostOverlayServicesProvider,
    ) -> Rc<dyn IHostOverlayServices> {
        provider.get_services_for_dockable(host, dockable)
    }
    /// Gets the resolved screen's busy service.
    pub fn get_busy_service(
        host: Rc<dyn IScreen>,
        provider: &dyn IHostOverlayServicesProvider,
    ) -> Rc<dyn IDockBusyService> {
        Self::get_overlay_services(host, provider).busy()
    }
    /// Gets the busy service for the current dockable context.
    pub fn get_busy_service_for_dockable(
        host: Rc<dyn IScreen>,
        dockable: Rc<dyn IDockable>,
        provider: &dyn IHostOverlayServicesProvider,
    ) -> Rc<dyn IDockBusyService> {
        Self::get_overlay_services_for_dockable(host, dockable, provider).busy()
    }
    /// Gets the resolved screen's dialog service.
    pub fn get_dialog_service(
        host: Rc<dyn IScreen>,
        provider: &dyn IHostOverlayServicesProvider,
    ) -> Rc<dyn IDockDialogService> {
        Self::get_overlay_services(host, provider).dialogs()
    }
    /// Gets the dialog service for the current dockable context.
    pub fn get_dialog_service_for_dockable(
        host: Rc<dyn IScreen>,
        dockable: Rc<dyn IDockable>,
        provider: &dyn IHostOverlayServicesProvider,
    ) -> Rc<dyn IDockDialogService> {
        Self::get_overlay_services_for_dockable(host, dockable, provider).dialogs()
    }
    /// Gets the resolved screen's confirmation service.
    pub fn get_confirmation_service(
        host: Rc<dyn IScreen>,
        provider: &dyn IHostOverlayServicesProvider,
    ) -> Rc<dyn IDockConfirmationService> {
        Self::get_overlay_services(host, provider).confirmations()
    }
    /// Gets the confirmation service for the current dockable context.
    pub fn get_confirmation_service_for_dockable(
        host: Rc<dyn IScreen>,
        dockable: Rc<dyn IDockable>,
        provider: &dyn IHostOverlayServicesProvider,
    ) -> Rc<dyn IDockConfirmationService> {
        Self::get_overlay_services_for_dockable(host, dockable, provider).confirmations()
    }
    /// Executes the actual router NavigateBack only when more than one item exists.
    pub fn try_navigate_back(host: Rc<dyn IScreen>) -> bool {
        if host.router().navigation_stack().count() <= 1 {
            return false;
        }
        let _ = host
            .router()
            .navigate_back()
            .execute_default()
            .subscribe_next(|_| {});
        true
    }
    /// Calls the nearest factory's CloseDockable. True means a factory was found;
    /// source close cancellation is not converted into a false helper result.
    pub fn try_close_dockable(dockable: Rc<dyn IDockable>) -> bool {
        let Some(factory) = Self::find_factory(dockable.clone()) else {
            return false;
        };
        factory.close_dockable(Some(dockable));
        true
    }
    /// Finds the nearest factory, preserving the source's initial direct check and
    /// subsequent owner loop (including its second first-node read after a null).
    pub fn find_factory(dockable: Rc<dyn IDockable>) -> Option<Rc<dyn IFactory>> {
        if let Some(factory) = dockable.factory() {
            return Some(factory);
        }
        let mut current = Some(dockable);
        while let Some(item) = current {
            if let Some(factory) = item.factory() {
                return Some(factory);
            }
            current = item.owner();
        }
        None
    }
}
