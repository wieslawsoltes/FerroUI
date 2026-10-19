//! `FlattenBufferResult` (`Cache/ObservableCacheEx.FlattenBufferResult.cs`): the operator of the
//! sequences of buffers of change sets, which `Buffer` makes.

use crate::cache::{ChangeSet, IChangeSet};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;

/// The buffers of change sets, each as one change set; the empty buffers are left out
/// (`FlattenBufferResult(IObservable<IList<IChangeSet<TObject, TKey>>>)`).
pub fn flatten_buffer_result<T: Clone + 'static, K: Clone + 'static, C: IChangeSet<T, K>>(source: Rc<dyn IObservable<Vec<C>>>) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    source
        .where_(|buffer: &Vec<C>| !buffer.is_empty())
        .select(|updates: Vec<C>| updates.iter().flat_map(|update| update.change_set().iter().cloned()).collect::<ChangeSet<T, K>>())
}

/// [`flatten_buffer_result`] as a method of the sequences of buffers of change sets of caches.
pub trait FlattenBufferResultCacheEx<T, K> {
    /// The buffers of change sets, each as one change set (`FlattenBufferResult`).
    fn flatten_buffer_result(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>>;
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, C: IChangeSet<T, K>> FlattenBufferResultCacheEx<T, K> for Rc<dyn IObservable<Vec<C>>> {
    fn flatten_buffer_result(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        flatten_buffer_result(self.clone())
    }
}
