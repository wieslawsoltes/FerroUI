// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferrodock_model_reactiveui::controls::Tool;
use ferrodock_model_reactiveui::core::DockableBaseImpl;
use ferrodock_model_reactiveui::impl_dockable;
use ferroui_reactiveui::{reactive_object, IRoutableViewModel, IScreen, RoutingState};
use std::any::TypeId;
use std::rc::{Rc, Weak};
/// Source tool with an independent nested ReactiveUI router and immutable route metadata.
pub struct RoutableTool {
    base: Tool,
    router: Rc<RoutingState>,
    host: Weak<dyn IScreen>,
    url: String,
}
reactive_object!(RoutableTool, base);
impl DockableBaseImpl for RoutableTool {}
impl_dockable!(@base RoutableTool,Tool,
    fn as_local_target(self:Rc<Self>)->Option<Rc<dyn ILocalTarget>> { Some(self) },
    fn as_tool(self:Rc<Self>)->Option<Rc<dyn ITool>> { Some(self) },
    fn as_document(self:Rc<Self>)->Option<Rc<dyn IDocument>> { Some(self) },
    fn as_mdi_document(self:Rc<Self>)->Option<Rc<dyn IMdiDocument>> { Some(self) },
    fn as_docking_window_state(&self)->Option<&dyn IDockingWindowState> { Some(self) },
    fn boxed_interface(self:Rc<Self>,target:TypeId)->Option<BoxedValue> {
        if target==TypeId::of::<dyn IScreen>() { return Some(Rc::new(self as Rc<dyn IScreen>)); }
        if target==TypeId::of::<dyn IRoutableViewModel>() { return Some(Rc::new(self as Rc<dyn IRoutableViewModel>)); } None
    }
);
impl ILocalTarget for RoutableTool {}
impl IDocument for RoutableTool {}
impl ITool for RoutableTool {}
document_contracts!(RoutableTool);
impl RoutableTool {
    /// Creates the source model, keeping an empty explicit URL distinct from no URL.
    pub fn new(host: Rc<dyn IScreen>, url: Option<String>) -> Rc<Self> {
        Rc::new_cyclic(|runtime| Self::construct(runtime.clone(), host, url))
    }
    /// Constructs the component within a complete enclosing routable screen model.
    /// Host ownership follows the shared ReactiveUI weak-host convention.
    pub fn construct<
        T: DockableBaseImpl
            + ITool
            + IMdiDocument
            + IDockingWindowState
            + IScreen
            + IRoutableViewModel
            + 'static,
    >(
        runtime: Weak<T>,
        host: Rc<dyn IScreen>,
        url: Option<String>,
    ) -> Self {
        Self {
            base: Tool::construct(runtime),
            router: RoutingState::new(),
            host: Rc::downgrade(&host),
            url: url.unwrap_or_else(super::class_name::<T>),
        }
    }
    /// Gets the same nested router reference.
    pub fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }
    /// Gets the immutable route segment.
    pub fn url_path_segment(&self) -> String {
        self.url.clone()
    }
    /// Gets the original host while it remains alive.
    pub fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host.upgrade()
    }
    /// Registers this native model's screen/dockable/routing projections in a composition.
    pub fn register_types(self: &Rc<Self>, types: &crate::overlays::hosting::HostTypeRegistry) {
        types.register_dockable_screen(self);
        types.register_routable(self);
    }
}
impl IScreen for RoutableTool {
    fn router(&self) -> Rc<RoutingState> {
        self.router()
    }
}
impl IRoutableViewModel for RoutableTool {
    fn url_path_segment(&self) -> Option<String> {
        Some(self.url_path_segment())
    }
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host_screen()
    }
}
