// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::overlays::hosting::HostTypeRegistry;
use ferrodock_model::controls::IRootDock;
use ferrodock_model::core::*;
use ferrodock_model::services::*;
use ferroui_reactiveui::{
    reactive_object, IReactiveObjectExt, IRoutableViewModel, IScreen, ReactiveObject,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
/// Source view model creating/initializing its layout and forwarding overlay services.
pub struct DockViewModel {
    reactive: ReactiveObject,
    factory: Rc<dyn IFactory>,
    layout: RefCell<Option<Rc<dyn IRootDock>>>,
    host: Weak<dyn IScreen>,
    types: Rc<HostTypeRegistry>,
}
reactive_object!(DockViewModel);
impl DockViewModel {
    /// Creates and initializes the source factory's layout, with the route segment "dock".
    pub fn new(host: Rc<dyn IScreen>, factory: Rc<dyn IFactory>) -> Rc<Self> {
        Self::with_types(host, factory, HostTypeRegistry::new())
    }
    /// Uses native source interface projections shared by the application composition.
    pub fn with_types(
        host: Rc<dyn IScreen>,
        factory: Rc<dyn IFactory>,
        types: Rc<HostTypeRegistry>,
    ) -> Rc<Self> {
        let model = Rc::new(Self {
            reactive: ReactiveObject::new(),
            factory,
            layout: RefCell::new(None),
            host: Rc::downgrade(&host),
            types,
        });
        let layout = model.factory.create_layout();
        if let Some(layout) = &layout {
            model.factory.init_layout(layout.clone());
        }
        model.set_layout(layout);
        model
    }
    /// Gets the current shared layout.
    pub fn layout(&self) -> Option<Rc<dyn IRootDock>> {
        self.layout.borrow().clone()
    }
    /// Sets Layout with original INPC equality/notifications; the setter does not
    /// implicitly initialize replacement layouts.
    pub fn set_layout(&self, value: Option<Rc<dyn IRootDock>>) {
        self.raise_and_set_if_changed(&self.layout, value, "Layout");
    }
    /// Gets the native projections used to discover layout overlay membership.
    pub fn types(&self) -> Rc<HostTypeRegistry> {
        self.types.clone()
    }
    fn overlays(&self) -> Rc<dyn IHostOverlayServices> {
        self.layout()
            .and_then(|root| {
                self.types
                    .dockable::<dyn IHostOverlayServices>(&(root as Rc<dyn IDockable>))
            })
            .expect("Overlay services are not available.")
    }
}
impl IRoutableViewModel for DockViewModel {
    fn url_path_segment(&self) -> Option<String> {
        Some("dock".into())
    }
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host.upgrade()
    }
}
impl IHostOverlayServices for DockViewModel {
    fn busy(&self) -> Rc<dyn IDockBusyService> {
        self.overlays().busy()
    }
    fn dialogs(&self) -> Rc<dyn IDockDialogService> {
        self.overlays().dialogs()
    }
    fn confirmations(&self) -> Rc<dyn IDockConfirmationService> {
        self.overlays().confirmations()
    }
    fn global_busy_service(&self) -> Rc<dyn IDockGlobalBusyService> {
        self.overlays().global_busy_service()
    }
    fn global_dialog_service(&self) -> Rc<dyn IDockGlobalDialogService> {
        self.overlays().global_dialog_service()
    }
    fn global_confirmation_service(&self) -> Rc<dyn IDockGlobalConfirmationService> {
        self.overlays().global_confirmation_service()
    }
}
