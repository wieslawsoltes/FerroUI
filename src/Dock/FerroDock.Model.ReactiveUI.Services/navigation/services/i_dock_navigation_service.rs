// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::{IDockable, IFactory};
use ferroui_reactiveui::IScreen;
use std::rc::Rc;
/// Opens documents in the supplied screen's current document docking context.
pub trait IDockNavigationService {
    /// Attaches a fallback factory and host screen.
    fn attach_factory(&self, factory: Rc<dyn IFactory>, host: Rc<dyn IScreen>);
    /// Opens or reactivates a document, optionally floating it through the factory.
    fn open_document(
        &self,
        host_screen: Rc<dyn IScreen>,
        document: Rc<dyn IDockable>,
        float_window: bool,
    );
}
