//! `FlattenBufferResult` (`List/ObservableListEx.FlattenBufferResult.cs`): the operator of the
//! sequences of buffers of change sets of lists.

use crate::list::{ChangeSet, IChangeSet};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// The buffers of change sets, each as one change set; the empty buffers are left out
/// (`FlattenBufferResult(IObservable<IList<IChangeSet<T>>>)`).
pub fn flatten_buffer_result<T: Clone + 'static, C: IChangeSet<T>>(source: Rc<dyn IObservable<Vec<C>>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    source
        .where_(|buffer: &Vec<C>| !buffer.is_empty())
        .select(|updates: Vec<C>| updates.iter().flat_map(|update| update.change_set().iter().cloned()).collect::<ChangeSet<T>>())
}

/// [`flatten_buffer_result`] as a method of the sequences of buffers of change sets of lists.
pub trait FlattenBufferResultListEx<T> {
    /// The buffers of change sets, each as one change set (`FlattenBufferResult`).
    fn flatten_buffer_result(&self) -> Rc<dyn IObservable<ChangeSet<T>>>;
}

impl<T: Clone + 'static, C: IChangeSet<T>> FlattenBufferResultListEx<T> for Rc<dyn IObservable<Vec<C>>> {
    fn flatten_buffer_result(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        flatten_buffer_result(self.clone())
    }
}
