//! `IWatcher<TObject, TKey>` (`Experimental/IWatcher.cs`).

use crate::cache::Change;
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// A watcher of the changes of the items of a cache by key (`IWatcher<TObject, TKey>`).
pub trait IWatcher<T, K>: IDisposable {
    /// The changes of the item of `key`, starting with its addition when it is in the cache
    /// (`Watch(TKey)`).
    fn watch(&self, key: K) -> Rc<dyn IObservable<Change<T, K>>>;
}
