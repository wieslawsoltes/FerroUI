//! `SortAndVirtualize<TObject, TKey>` (`Cache/Internal/SortAndVirtualize.cs`).

use super::sorted_key_value_applicator::{ApplicatorOptions, SortedKeyValueApplicator};
use super::{Comparer, KeyComparer, KeyValueComparer};
use crate::cache::{Change, ChangeReason, ChangeSet, ChangeSetWithContext, ObservableCacheEx, SortAndVirtualizeOptions, VirtualContext, VirtualRequest, VirtualResponse};
use crate::list::SortException;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::Hash;
use std::rc::Rc;

type VirtualChanges<T, K> = ChangeSetWithContext<T, K, VirtualContext<T>>;

/// The window of the items of `source`, sorted by the latest comparer of `comparer_changed`, that
/// the latest request of `virtual_requests` asks for (`SortAndVirtualize<TObject, TKey>.Run`).
/// The change sets hold the changes of the window only, without indexes, with the window, the
/// comparer and the options as their context.
///
/// Deviation (DEVIATIONS.md, DynamicData): the sources are merged without the shared delivery
/// queue of upstream (single-threaded, section 5.1); a `SortException` of the sorted list ends
/// the sequence with that error.
pub(crate) fn sort_and_virtualize<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    comparer_changed: Rc<dyn IObservable<Comparer<T>>>,
    virtual_requests: Rc<dyn IObservable<VirtualRequest>>,
    options: SortAndVirtualizeOptions,
) -> Rc<dyn IObservable<VirtualChanges<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<VirtualChanges<T, K>>>| {
        let state = Rc::new(RefCell::new(State::<T, K>::new(options)));

        // Maintains the sorted list of pairs.
        let comparer_state = state.clone();
        let comparer_changed = comparer_changed.select(move |comparer: Comparer<T>| comparer_state.borrow_mut().change_comparer(comparer));

        let params_state = state.clone();
        let params_changed = virtual_requests
            .distinct_until_changed()
            // Exclude the dodgy parameters (a negative start index cannot be made).
            .where_(|parameters: &VirtualRequest| parameters.size() > 0)
            .select(move |request: VirtualRequest| params_state.borrow_mut().change_request(request));

        let data_state = state.clone();
        let data_change = source
            // Each change set must have unique keys only, otherwise the calculation of the
            // virtual changes is very complex.
            .ensure_unique_keys()
            .try_select(move |changes: ChangeSet<T, K>| data_state.borrow_mut().process_changes(&changes).map_err(SortException::into_error));

        Observable::merge_all(vec![comparer_changed, params_changed, data_change])
            .where_(|changes: &VirtualChanges<T, K>| changes.count() != 0)
            .subscribe(observer)
    })
}

/// The state of one subscription of [`sort_and_virtualize`].
struct State<T, K> {
    options: SortAndVirtualizeOptions,
    virtual_params: VirtualRequest,
    virtual_items: Vec<(K, T)>,
    comparer: Option<Comparer<T>>,
    applicator: Option<SortedKeyValueApplicator<T, K>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> State<T, K> {
    fn new(options: SortAndVirtualizeOptions) -> Self {
        let virtual_params = VirtualRequest::DEFAULT;
        Self { options, virtual_params, virtual_items: Vec::with_capacity(virtual_params.size()), comparer: None, applicator: None }
    }

    fn change_comparer(&mut self, comparer: Comparer<T>) -> VirtualChanges<T, K> {
        self.comparer = Some(comparer.clone());
        let key_value_comparer = KeyValueComparer::new(Some(comparer));
        match &mut self.applicator {
            None => {
                let sort_options = ApplicatorOptions { reset_threshold: self.options.reset_threshold, use_binary_search: self.options.use_binary_search };
                self.applicator = Some(SortedKeyValueApplicator::new(Vec::with_capacity(self.options.initial_capacity), key_value_comparer, sort_options));
            }
            Some(applicator) => applicator.change_comparer(key_value_comparer),
        }
        self.apply_virtual_changes(None)
    }

    fn change_request(&mut self, request: VirtualRequest) -> VirtualChanges<T, K> {
        self.virtual_params = request;
        // The comparer has not been received yet.
        if self.applicator.is_none() {
            return ChangeSetWithContext::empty(VirtualContext::empty());
        }
        // Apply the virtual changes again.
        self.apply_virtual_changes(None)
    }

    fn process_changes(&mut self, changes: &ChangeSet<T, K>) -> Result<VirtualChanges<T, K>, SortException> {
        // The comparer has not been received yet.
        let Some(applicator) = &mut self.applicator else {
            return Ok(ChangeSetWithContext::empty(VirtualContext::empty()));
        };
        // Apply the changes to the sorted list, then the virtual changes again.
        applicator.process_changes(changes)?;
        Ok(self.apply_virtual_changes(Some(changes)))
    }

    fn apply_virtual_changes(&mut self, change_set: Option<&ChangeSet<T, K>>) -> VirtualChanges<T, K> {
        let sorted_list = self.applicator.as_ref().map_or(&[][..], |applicator| applicator.target());
        let previous_virtual_list = std::mem::take(&mut self.virtual_items);

        // Calculate the virtual changes again.
        let mut current_virtual_items = Vec::with_capacity(self.virtual_params.size());
        current_virtual_items.extend(sorted_list.iter().skip(self.virtual_params.start_index()).take(self.virtual_params.size()).cloned());

        let response_params = VirtualResponse::new(self.virtual_params.size(), self.virtual_params.start_index(), sorted_list.len());
        let comparer = self.comparer.clone().expect("the comparer comes with the applicator");
        let context = VirtualContext::new(response_params, comparer, self.options);

        // Calculate the notifications.
        let virtual_changes = calculate_virtual_changes(context, &current_virtual_items, &previous_virtual_list, change_set);
        self.virtual_items = current_virtual_items;
        virtual_changes
    }
}

/// The changes within the window (`CalculateVirtualChanges`).
fn calculate_virtual_changes<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    context: VirtualContext<T>,
    current_items: &[(K, T)],
    previous_items: &[(K, T)],
    changes: Option<&ChangeSet<T, K>>,
) -> VirtualChanges<T, K> {
    let mut result = Vec::with_capacity(current_items.len() * 2);
    result.extend(KeyComparer::except(previous_items, current_items).into_iter().map(|(key, value)| Change::new(ChangeReason::Remove, key, value)));
    result.extend(KeyComparer::except(current_items, previous_items).into_iter().map(|(key, value)| Change::new(ChangeReason::Add, key, value)));

    match changes {
        None => {
            // The comparer has changed, so a refresh is sent for each item in the same window.
            for (key, value) in KeyComparer::intersect(previous_items, current_items) {
                result.push(Change::new(ChangeReason::Refresh, key, value));
            }
        }
        Some(changes) => {
            let key_in_previous_and_current: HashSet<K> = KeyComparer::intersect(previous_items, current_items).into_iter().map(|(key, _)| key).collect();
            for change in changes {
                // An update (or a refresh) can only occur if it was in the previous and the
                // current result sets; in only one of them, it is an add or a remove.
                if !key_in_previous_and_current.contains(change.key()) {
                    continue;
                }
                if matches!(change.reason(), ChangeReason::Update | ChangeReason::Refresh) {
                    result.push(change.clone());
                }
            }
        }
    }

    ChangeSetWithContext::new(ChangeSet::from(result), context)
}
