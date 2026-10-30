// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::ConfirmationRequest;
use crate::collections::ReadOnlyObservableCollection;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::threading::tasks::Task;
use std::rc::Rc;
/// Source per-host confirmation service contract.
pub trait IDockConfirmationService: INotifyPropertyChanged {
    /// Gets the same live read-only request collection.
    fn confirmations(&self) -> ReadOnlyObservableCollection<Rc<ConfirmationRequest>>;
    /// Gets the active request.
    fn active_confirmation(&self) -> Option<Rc<ConfirmationRequest>>;
    /// Gets whether any requests are present.
    fn has_confirmations(&self) -> bool;
    /// Shows a confirmation with explicit labels.
    fn confirm_async(
        &self,
        title: String,
        message: String,
        confirm_text: String,
        cancel_text: String,
    ) -> Task<bool>;
    /// Shows a confirmation using source Confirm/Cancel default labels.
    fn confirm_async_default(&self, title: String, message: String) -> Task<bool> {
        self.confirm_async(title, message, "Confirm".into(), "Cancel".into())
    }
    /// Closes a request, including a request no longer present in this service.
    fn close(&self, request: Rc<ConfirmationRequest>, result: bool);
    /// Cancels a snapshot of the currently pending requests.
    fn cancel_all(&self);
}

impl PartialEq for dyn IDockConfirmationService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
