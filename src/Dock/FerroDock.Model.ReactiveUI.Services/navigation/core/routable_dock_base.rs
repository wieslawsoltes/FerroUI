// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDock;
use ferrodock_model_reactiveui::core::{DockBase, DockableBaseImpl};
use ferroui_reactiveui::{reactive_object, IRoutableViewModel, IScreen};
use std::rc::{Rc, Weak};
/// Source abstract DockBase plus immutable IRoutableViewModel metadata.
/// This component does not add an IScreen or allocate a routing state.
pub struct RoutableDockBase {
    base: DockBase,
    host: Weak<dyn IScreen>,
    url: String,
}
reactive_object!(RoutableDockBase, base);
impl std::ops::Deref for RoutableDockBase {
    type Target = DockBase;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl RoutableDockBase {
    /// Constructs within a complete routable dock. Initialize inherited DockBase
    /// observations after Rc construction, just as for other abstract dock components.
    pub fn construct<T: DockableBaseImpl + IDock + IRoutableViewModel + 'static>(
        runtime: Weak<T>,
        host: Rc<dyn IScreen>,
        url: Option<String>,
    ) -> Self {
        Self {
            base: DockBase::construct(runtime),
            host: Rc::downgrade(&host),
            url: url.unwrap_or_else(super::super::controls::class_name::<T>),
        }
    }
    /// Gets the immutable source route segment.
    pub fn url_path_segment(&self) -> String {
        self.url.clone()
    }
    /// Gets the original host while alive, following shared ReactiveUI ownership.
    pub fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host.upgrade()
    }
}
impl IRoutableViewModel for RoutableDockBase {
    fn url_path_segment(&self) -> Option<String> {
        Some(self.url_path_segment())
    }
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host_screen()
    }
}

impl AsRef<ferroui_reactiveui::ReactiveObject> for RoutableDockBase {
    fn as_ref(&self) -> &ferroui_reactiveui::ReactiveObject {
        self.base.as_ref()
    }
}
