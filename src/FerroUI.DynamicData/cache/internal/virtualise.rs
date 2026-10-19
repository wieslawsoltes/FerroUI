//! `Virtualise<TObject, TKey>` (`Cache/Internal/Virtualise.cs`).

use super::sort::where_some;
use super::{filtered_index_calculator, KeyValueCollection};
use crate::cache::{ChangeSet, SortReason, SortedChangeSet, VirtualChangeSet, VirtualRequest, VirtualResponse};
use crate::list::SortException;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The window of the sorted change sets of `source` that the latest request of
/// `virtual_requests` asks for (`Virtualise<TObject, TKey>.Run`).
///
/// Deviation (DEVIATIONS.md, DynamicData): the requests and the data are merged without the
/// shared delivery queue of upstream (single-threaded, section 5.1).
pub(crate) fn virtualise<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<SortedChangeSet<T, K>>>,
    virtual_requests: Rc<dyn IObservable<VirtualRequest>>,
) -> Rc<dyn IObservable<VirtualChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<VirtualChangeSet<T, K>>>| {
        let virtualiser = Rc::new(RefCell::new(Virtualiser::new(None)));
        let request_virtualiser = virtualiser.clone();
        let request =
            virtual_requests.try_select(move |parameters: VirtualRequest| request_virtualiser.borrow_mut().virtualise_request(parameters).map_err(SortException::into_error));
        let data_virtualiser = virtualiser.clone();
        let data_change = source.try_select(move |updates: SortedChangeSet<T, K>| data_virtualiser.borrow_mut().update(&updates).map_err(SortException::into_error));
        where_some(Observable::merge_all(vec![request, data_change])).subscribe(observer)
    })
}

/// The state of one subscription of [`virtualise`] (`Virtualise<TObject, TKey>.Virtualiser`).
struct Virtualiser<T, K> {
    all: KeyValueCollection<T, K>,
    current: KeyValueCollection<T, K>,
    is_loaded: bool,
    parameters: VirtualRequest,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Virtualiser<T, K> {
    fn new(request: Option<VirtualRequest>) -> Self {
        Self { all: KeyValueCollection::empty(), current: KeyValueCollection::empty(), is_loaded: false, parameters: request.unwrap_or_default() }
    }

    /// The window after a change of the data (`Update`).
    fn update(&mut self, updates: &SortedChangeSet<T, K>) -> Result<Option<VirtualChangeSet<T, K>>, SortException> {
        self.is_loaded = true;
        self.all = updates.sorted_items().clone();
        self.virtualise(Some(updates))
    }

    /// The window of a new request; nothing for an empty window or the request of now
    /// (`Virtualise(IVirtualRequest)`).
    fn virtualise_request(&mut self, parameters: VirtualRequest) -> Result<Option<VirtualChangeSet<T, K>>, SortException> {
        if parameters.size() < 1 {
            return Ok(None);
        }
        if parameters.size() == self.parameters.size() && parameters.start_index() == self.parameters.start_index() {
            return Ok(None);
        }
        self.parameters = parameters;
        self.virtualise(None)
    }

    fn virtualise(&mut self, updates: Option<&SortedChangeSet<T, K>>) -> Result<Option<VirtualChangeSet<T, K>>, SortException> {
        if !self.is_loaded {
            return Ok(None);
        }

        let previous = self.current.clone();
        let virtualised: Vec<(K, T)> = self.all.iter().skip(self.parameters.start_index()).take(self.parameters.size()).cloned().collect();
        let sort_reason = updates.map_or(SortReason::DataChanged, |updates| updates.sorted_items().sort_reason());
        self.current = KeyValueCollection::new(virtualised, self.all.comparer().clone(), sort_reason, self.all.optimisations());

        // Check for changes within the current window; notify when there are changes, or when the
        // number of items has not changed.
        let notifications = filtered_index_calculator::calculate(&self.current, &previous, updates.map(|updates| updates.changes()))?;
        if notifications.is_empty() && previous.count() != self.current.count() {
            return Ok(None);
        }

        let response = VirtualResponse::new(self.parameters.size(), self.parameters.start_index(), self.all.count());
        Ok(Some(VirtualChangeSet::new(ChangeSet::from(notifications), self.current.clone(), response)))
    }
}
