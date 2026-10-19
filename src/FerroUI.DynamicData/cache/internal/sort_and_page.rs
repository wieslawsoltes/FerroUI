//! `SortAndPage<TObject, TKey>` (`Cache/Internal/SortAndPage.cs`).

use super::sorted_key_value_applicator::{ApplicatorOptions, SortedKeyValueApplicator};
use super::{Comparer, KeyComparer, KeyValueComparer};
use crate::cache::{Change, ChangeReason, ChangeSet, ChangeSetWithContext, ObservableCacheEx, PageContext, PageRequest, PageResponse, SortAndPageOptions};
use crate::list::SortException;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::Hash;
use std::rc::Rc;

type PagedChanges<T, K> = ChangeSetWithContext<T, K, PageContext<T>>;

/// The page of the items of `source`, sorted by the latest comparer of `comparer_changed`, that
/// the latest request of `page_requests` asks for (`SortAndPage<TObject, TKey>.Run`). The change
/// sets hold the changes of the page only, without indexes, with the page, the comparer and the
/// options as their context.
///
/// Deviation (DEVIATIONS.md, DynamicData): the sources are merged without the shared delivery
/// queue of upstream (single-threaded, section 5.1); a `SortException` of the sorted list ends
/// the sequence with that error.
pub(crate) fn sort_and_page<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    comparer_changed: Rc<dyn IObservable<Comparer<T>>>,
    page_requests: Rc<dyn IObservable<PageRequest>>,
    options: SortAndPageOptions,
) -> Rc<dyn IObservable<PagedChanges<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<PagedChanges<T, K>>>| {
        let state = Rc::new(RefCell::new(State::<T, K>::new(options)));

        // Maintains the sorted list of pairs.
        let comparer_state = state.clone();
        let comparer_changed = comparer_changed.select(move |comparer: Comparer<T>| comparer_state.borrow_mut().change_comparer(comparer));

        let params_state = state.clone();
        let params_changed = page_requests
            .distinct_until_changed()
            // Exclude the dodgy parameters.
            .where_(|parameters: &PageRequest| parameters.page() > 0 && parameters.size() > 0)
            .select(move |request: PageRequest| params_state.borrow_mut().change_request(request));

        let data_state = state.clone();
        let data_change = source
            // Each change set must have unique keys only, otherwise the calculation of the paged
            // changes is very complex.
            .ensure_unique_keys()
            .try_select(move |changes: ChangeSet<T, K>| data_state.borrow_mut().process_changes(&changes).map_err(SortException::into_error));

        Observable::merge_all(vec![
            comparer_changed.skip(1),
            params_changed.where_(|changes: &PagedChanges<T, K>| changes.count() != 0),
            data_change.where_(|changes: &PagedChanges<T, K>| changes.count() != 0),
        ])
        .subscribe(observer)
    })
}

/// The state of one subscription of [`sort_and_page`].
struct State<T, K> {
    options: SortAndPageOptions,
    page_request: PageRequest,
    paged_items: Vec<(K, T)>,
    comparer: Option<Comparer<T>>,
    applicator: Option<SortedKeyValueApplicator<T, K>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> State<T, K> {
    fn new(options: SortAndPageOptions) -> Self {
        let page_request = PageRequest::DEFAULT;
        Self { options, page_request, paged_items: Vec::with_capacity(page_request.size()), comparer: None, applicator: None }
    }

    fn change_comparer(&mut self, comparer: Comparer<T>) -> PagedChanges<T, K> {
        self.comparer = Some(comparer.clone());
        let key_value_comparer = KeyValueComparer::new(Some(comparer));
        match &mut self.applicator {
            None => {
                let sort_options = ApplicatorOptions { reset_threshold: self.options.reset_threshold, use_binary_search: self.options.use_binary_search };
                self.applicator = Some(SortedKeyValueApplicator::new(Vec::with_capacity(self.options.initial_capacity), key_value_comparer, sort_options));
            }
            Some(applicator) => applicator.change_comparer(key_value_comparer),
        }
        self.apply_paged_changes(None)
    }

    fn change_request(&mut self, request: PageRequest) -> PagedChanges<T, K> {
        self.page_request = request;
        // The comparer has not been received yet.
        if self.applicator.is_none() {
            return ChangeSetWithContext::empty(PageContext::empty());
        }
        // Apply the paged changes again.
        self.apply_paged_changes(None)
    }

    fn process_changes(&mut self, changes: &ChangeSet<T, K>) -> Result<PagedChanges<T, K>, SortException> {
        // The comparer has not been received yet.
        let Some(applicator) = &mut self.applicator else {
            return Ok(ChangeSetWithContext::empty(PageContext::empty()));
        };
        // Apply the changes to the sorted list, then the paged changes again.
        applicator.process_changes(changes)?;
        Ok(self.apply_paged_changes(Some(changes)))
    }

    fn apply_paged_changes(&mut self, change_set: Option<&ChangeSet<T, K>>) -> PagedChanges<T, K> {
        let sorted_list = self.applicator.as_ref().map_or(&[][..], |applicator| applicator.target());
        let previous_virtual_list = std::mem::take(&mut self.paged_items);
        let list_size = sorted_list.len();
        let pages = calculate_pages(&self.page_request, list_size);
        let page = if self.page_request.page() > pages { pages } else { self.page_request.page() };
        let skip = self.page_request.size() * page.saturating_sub(1);

        // Calculate the paged changes again.
        let mut current_pages_items = Vec::with_capacity(self.page_request.size());
        current_pages_items.extend(sorted_list.iter().skip(skip).take(self.page_request.size()).cloned());

        let response_params = PageResponse::new(self.page_request.size(), list_size, page, pages);
        let comparer = self.comparer.clone().expect("the comparer comes with the applicator");
        let context = PageContext::new(response_params, comparer, self.options);

        // Calculate the notifications.
        let virtual_changes = calculate_page_changes(context, &current_pages_items, &previous_virtual_list, change_set);
        self.paged_items = current_pages_items;
        virtual_changes
    }
}

fn calculate_pages(request: &PageRequest, total_count: usize) -> usize {
    if request.size() >= total_count {
        return 1;
    }
    let pages = total_count / request.size();
    let overlap = total_count % request.size();
    if overlap == 0 {
        return pages;
    }
    pages + 1
}

/// The changes within the page (`CalculatePageChanges`).
fn calculate_page_changes<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    context: PageContext<T>,
    current_items: &[(K, T)],
    previous_items: &[(K, T)],
    changes: Option<&ChangeSet<T, K>>,
) -> PagedChanges<T, K> {
    let mut result = Vec::with_capacity(current_items.len() * 2);
    result.extend(KeyComparer::except(previous_items, current_items).into_iter().map(|(key, value)| Change::new(ChangeReason::Remove, key, value)));
    result.extend(KeyComparer::except(current_items, previous_items).into_iter().map(|(key, value)| Change::new(ChangeReason::Add, key, value)));

    if let Some(changes) = changes {
        let key_in_previous_and_current: HashSet<K> = KeyComparer::intersect(previous_items, current_items).into_iter().map(|(key, _)| key).collect();
        for change in changes {
            // An update (or a refresh) can only occur if it was in the previous and the current
            // result sets; in only one of them, it is an add or a remove.
            if !key_in_previous_and_current.contains(change.key()) {
                continue;
            }
            if matches!(change.reason(), ChangeReason::Update | ChangeReason::Refresh) {
                result.push(change.clone());
            }
        }
    }

    ChangeSetWithContext::new(ChangeSet::from(result), context)
}
