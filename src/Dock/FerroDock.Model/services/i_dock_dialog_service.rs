// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::{DialogContent, DialogRequest, DialogResult};
use crate::collections::ReadOnlyObservableCollection;
use crate::BoxedValue;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::threading::tasks::Task;
use std::rc::Rc;
/// Object-safe source dialog service, with its generic result cast in the extension trait.
pub trait IDockDialogService: INotifyPropertyChanged {
    /// Gets the live read-only requests.
    fn dialogs(&self) -> ReadOnlyObservableCollection<Rc<DialogRequest>>;
    /// Gets the active request.
    fn active_dialog(&self) -> Option<Rc<DialogRequest>>;
    /// Gets whether requests are present.
    fn has_dialogs(&self) -> bool;
    /// Native specialization of ShowAsync<object>, retaining the original nullable result.
    fn show_async_boxed(
        &self,
        content: DialogContent,
        title: Option<String>,
    ) -> Task<Option<BoxedValue>>;
    /// Closes a request with a nullable shared result.
    fn close(&self, request: Rc<DialogRequest>, result: Option<BoxedValue>);
    /// Cancels a snapshot of currently pending dialogs.
    fn cancel_all(&self);
}
/// Generic source ShowAsync<T> available on concrete and erased service handles.
pub trait DockDialogServiceExt: IDockDialogService {
    /// Shows the original content and applies the source type/default result rule.
    fn show_async<T: DialogResult>(
        &self,
        content: impl Into<DialogContent>,
        title: Option<String>,
    ) -> Task<T> {
        let task = self.show_async_boxed(content.into(), title);
        Task::spawn_local(async move { Ok(T::from_dialog_result(task.await?)) })
    }
}
impl<S: IDockDialogService + ?Sized> DockDialogServiceExt for S {}

impl PartialEq for dyn IDockDialogService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
