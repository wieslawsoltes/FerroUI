// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_reactiveui::{IActivatableViewModel, IViewFor};
use ferroui_system::IDisposable;
use std::rc::Rc;
/// Bridges view activation to the view model's shared ReactiveUI activator.
pub trait IActivationBridge {
    /// Opens one activation scope for the supplied model and view. Explicitly
    /// disposing the result matches that activation with a deactivation.
    fn track(
        &self,
        view_model: Rc<dyn IActivatableViewModel>,
        view: Rc<dyn IViewFor>,
    ) -> Rc<dyn IDisposable>;
}
