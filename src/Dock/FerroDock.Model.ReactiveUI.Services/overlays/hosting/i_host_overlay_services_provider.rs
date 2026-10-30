// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{HostOverlayServicesProvider, HostTypeRegistry};
use ferrodock_model::core::IDockable;
use ferrodock_model::services::IHostOverlayServices;
use ferroui_reactiveui::IScreen;
use std::rc::Rc;
/// Source overlay service provider for a screen or current dockable context.
pub trait IHostOverlayServicesProvider {
    /// Resolves screen-scoped services or returns its cached fallback.
    fn get_services(&self, screen: Rc<dyn IScreen>) -> Rc<dyn IHostOverlayServices>;
    /// Resolves the dockable's current context before using the fallback screen.
    fn get_services_for_dockable(
        &self,
        screen: Rc<dyn IScreen>,
        dockable: Rc<dyn IDockable>,
    ) -> Rc<dyn IHostOverlayServices>;
    /// Native concrete-provider membership used by lifecycle's cache-only path.
    fn as_cached_provider(&self) -> Option<&HostOverlayServicesProvider> {
        None
    }
    /// Native reference projections shared by the service composition root.
    fn native_types(&self) -> Option<Rc<HostTypeRegistry>> {
        None
    }
}
