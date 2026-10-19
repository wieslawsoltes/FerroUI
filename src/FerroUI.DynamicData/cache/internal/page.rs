//! `Page<TObject, TKey>` (`Cache/Internal/Page.cs`).

use super::sort::where_some;
use super::{filtered_index_calculator, KeyValueCollection};
use crate::cache::{ChangeSet, PageRequest, PageResponse, PagedChangeSet, SortReason, SortedChangeSet};
use crate::list::SortException;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The page of the sorted change sets of `source` that the latest request of `page_requests` asks
/// for (`Page<TObject, TKey>.Run`).
///
/// Deviation (DEVIATIONS.md, DynamicData): the requests and the data are merged without the
/// shared delivery queue of upstream (single-threaded, section 5.1).
pub(crate) fn page<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<SortedChangeSet<T, K>>>,
    page_requests: Rc<dyn IObservable<PageRequest>>,
) -> Rc<dyn IObservable<PagedChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<PagedChangeSet<T, K>>>| {
        let paginator = Rc::new(RefCell::new(Paginator::new()));
        let request_paginator = paginator.clone();
        let request = page_requests.try_select(move |parameters: PageRequest| request_paginator.borrow_mut().paginate_request(parameters).map_err(SortException::into_error));
        let data_paginator = paginator.clone();
        let data_change = source.try_select(move |updates: SortedChangeSet<T, K>| data_paginator.borrow_mut().update(&updates).map_err(SortException::into_error));
        where_some(Observable::merge_all(vec![request, data_change])).subscribe(observer)
    })
}

/// The state of one subscription of [`page`] (`Page<TObject, TKey>.Paginator`).
struct Paginator<T, K> {
    all: KeyValueCollection<T, K>,
    current: KeyValueCollection<T, K>,
    is_loaded: bool,
    request: PageRequest,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Paginator<T, K> {
    fn new() -> Self {
        Self { all: KeyValueCollection::empty(), current: KeyValueCollection::empty(), is_loaded: false, request: PageRequest::DEFAULT }
    }

    /// The page of a new request; nothing for an empty page or the request of now (`Paginate(IPageRequest)`).
    fn paginate_request(&mut self, parameters: PageRequest) -> Result<Option<PagedChangeSet<T, K>>, SortException> {
        if parameters.size() < 1 {
            return Ok(None);
        }
        if parameters.size() == self.request.size() && parameters.page() == self.request.page() {
            return Ok(None);
        }
        self.request = parameters;
        self.paginate(None)
    }

    /// The page after a change of the data (`Update`).
    fn update(&mut self, updates: &SortedChangeSet<T, K>) -> Result<Option<PagedChangeSet<T, K>>, SortException> {
        self.is_loaded = true;
        self.all = updates.sorted_items().clone();
        self.paginate(Some(updates))
    }

    fn calculate_pages(&self) -> usize {
        if self.request.size() >= self.all.count() {
            return 1;
        }
        let pages = self.all.count() / self.request.size();
        let overlap = self.all.count() % self.request.size();
        if overlap == 0 {
            return pages;
        }
        pages + 1
    }

    fn paginate(&mut self, updates: Option<&SortedChangeSet<T, K>>) -> Result<Option<PagedChangeSet<T, K>>, SortException> {
        if !self.is_loaded {
            return Ok(None);
        }

        let previous = self.current.clone();
        let pages = self.calculate_pages();
        let page = if self.request.page() > pages { pages } else { self.request.page() };
        // A page 0, which upstream accepts, skips nothing (`Skip` of a negative count).
        let skip = self.request.size() * page.saturating_sub(1);
        let paged: Vec<(K, T)> = self.all.iter().skip(skip).take(self.request.size()).cloned().collect();
        let sort_reason = updates.map_or(SortReason::DataChanged, |updates| updates.sorted_items().sort_reason());
        self.current = KeyValueCollection::new(paged, self.all.comparer().clone(), sort_reason, self.all.optimisations());

        // Check for changes within the current page; notify when there are changes, or when the
        // number of items has not changed.
        let notifications = filtered_index_calculator::calculate(&self.current, &previous, updates.map(|updates| updates.changes()))?;
        if notifications.is_empty() && previous.count() != self.current.count() {
            return Ok(None);
        }

        let response = PageResponse::new(self.request.size(), self.all.count(), page, pages);
        Ok(Some(PagedChangeSet::new(self.current.clone(), ChangeSet::from(notifications), response)))
    }
}
