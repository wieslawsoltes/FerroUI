// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferrodock_model_reactiveui::controls::RootDock;
use ferrodock_model_reactiveui::core::DockableBaseImpl;
use ferrodock_model_reactiveui::impl_dock;
use ferroui_reactiveui::{reactive_object, ICommand, IRoutableViewModel, IScreen, RoutingState};
use std::any::TypeId;
use std::rc::{Rc, Weak};
/// Source root dock with nested routing and immutable route metadata.
pub struct RoutableRootDock {
    base: RootDock,
    router: Rc<RoutingState>,
    host: Weak<dyn IScreen>,
    url: String,
}
reactive_object!(RoutableRootDock, base);
impl DockableBaseImpl for RoutableRootDock {}
impl_dock!(@base RoutableRootDock,RootDock,
    fn as_root_dock(self:Rc<Self>)->Option<Rc<dyn IRootDock>> { Some(self) },
    fn boxed_interface(self:Rc<Self>,target:TypeId)->Option<BoxedValue> {
        if target==TypeId::of::<dyn IScreen>() { return Some(Rc::new(self as Rc<dyn IScreen>)); }
        if target==TypeId::of::<dyn IRoutableViewModel>() { return Some(Rc::new(self as Rc<dyn IRoutableViewModel>)); } None
    }
);
root_contracts!(RoutableRootDock);
impl RoutableRootDock {
    /// Creates the source routable root and initializes actual inherited dock observations.
    pub fn new(host: Rc<dyn IScreen>, url: Option<String>) -> Rc<Self> {
        let root = Rc::new_cyclic(|runtime| Self::construct(runtime, host, url));
        root.base.initialize(&root);
        root
    }
    /// Constructs the component for an enclosing complete routable root model.
    /// The caller must initialize inherited DockBase observations after construction.
    pub fn construct<T: DockableBaseImpl + IRootDock + IScreen + IRoutableViewModel + 'static>(
        runtime: &Weak<T>,
        host: Rc<dyn IScreen>,
        url: Option<String>,
    ) -> Self {
        Self {
            base: RootDock::construct(runtime),
            router: RoutingState::new(),
            host: Rc::downgrade(&host),
            url: url.unwrap_or_else(super::class_name::<T>),
        }
    }
    /// Gets the same nested router.
    pub fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }
    /// Gets the immutable route segment.
    pub fn url_path_segment(&self) -> String {
        self.url.clone()
    }
    /// Gets the original host while alive, following the shared ReactiveUI host convention.
    pub fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host.upgrade()
    }
    /// Registers source screen/dockable/root/routable projections in a native composition.
    pub fn register_types(self: &Rc<Self>, types: &crate::overlays::hosting::HostTypeRegistry) {
        types.register_dockable_screen(self);
        types.register_routable(self);
    }
}
impl IScreen for RoutableRootDock {
    fn router(&self) -> Rc<RoutingState> {
        self.router()
    }
}
impl IRoutableViewModel for RoutableRootDock {
    fn url_path_segment(&self) -> Option<String> {
        Some(self.url_path_segment())
    }
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host_screen()
    }
}
