// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::RoutableDocument;
use crate::navigation::services::DockNavigationHelpers;
use crate::overlays::hosting::IHostOverlayServicesProvider;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::services::*;
use ferrodock_model_reactiveui::core::DockableBaseImpl;
use ferroui_reactiveui::{reactive_object, IRoutableViewModel, IScreen};
use std::rc::{Rc, Weak};
/// Abstract source routable document component with overlay/navigation helpers.
/// Compose this into a concrete model and forward its source model contracts.
pub struct RoutableDocumentBase {
    base: RoutableDocument,
    provider: Option<Rc<dyn IHostOverlayServicesProvider>>,
    screen: Weak<dyn IScreen>,
    dockable: Weak<dyn IDockable>,
}
reactive_object!(RoutableDocumentBase, base);
impl std::ops::Deref for RoutableDocumentBase {
    type Target = RoutableDocument;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl RoutableDocumentBase {
    /// Constructs either source overload: None supplies no overlay provider.
    /// The route default uses the complete enclosing model's native class name.
    pub fn construct<
        T: DockableBaseImpl
            + IMdiDocument
            + IDockingWindowState
            + IScreen
            + IRoutableViewModel
            + 'static,
    >(
        runtime: Weak<T>,
        host: Rc<dyn IScreen>,
        provider: Option<Rc<dyn IHostOverlayServicesProvider>>,
        url: Option<String>,
    ) -> Self {
        Self {
            base: RoutableDocument::construct(runtime.clone(), host, url),
            provider,
            screen: runtime.clone(),
            dockable: runtime,
        }
    }
    fn get_overlay_services(&self) -> Rc<dyn IHostOverlayServices> {
        let provider = self
            .provider
            .as_ref()
            .expect("Overlay services provider is not available.");
        provider.get_services_for_dockable(
            self.screen.upgrade().expect("live routable model"),
            self.dockable.upgrade().expect("live routable model"),
        )
    }
    /// Source protected helper resolving this model as both screen and dockable.
    pub fn get_busy_service(&self) -> Rc<dyn IDockBusyService> {
        self.get_overlay_services().busy()
    }
    /// Source protected helper resolving this model as both screen and dockable.
    pub fn get_dialog_service(&self) -> Rc<dyn IDockDialogService> {
        self.get_overlay_services().dialogs()
    }
    /// Source protected helper resolving this model as both screen and dockable.
    pub fn get_confirmation_service(&self) -> Rc<dyn IDockConfirmationService> {
        self.get_overlay_services().confirmations()
    }
    /// Source protected helper executing this model's router, independently of HostScreen.
    pub fn navigate_back(&self) -> bool {
        DockNavigationHelpers::try_navigate_back(
            self.screen.upgrade().expect("live routable model"),
        )
    }
    /// Source protected helper resolving the owning factory and requesting close.
    pub fn close_dockable(&self) -> bool {
        DockNavigationHelpers::try_close_dockable(
            self.dockable.upgrade().expect("live routable model"),
        )
    }
}

impl AsRef<ferroui_reactiveui::ReactiveObject> for RoutableDocumentBase {
    fn as_ref(&self) -> &ferroui_reactiveui::ReactiveObject {
        self.base.as_ref()
    }
}
impl RoutableDocumentBase {
    /// Gets inherited window state without ambiguous interface method lookup.
    pub fn docking_state(&self) -> DockingWindowState {
        IDockable::docking_state(&self.base)
    }
    /// Sets inherited window state without ambiguous interface method lookup.
    pub fn set_docking_state(&self, value: DockingWindowState) {
        IDockable::set_docking_state(&self.base, value);
    }
}
