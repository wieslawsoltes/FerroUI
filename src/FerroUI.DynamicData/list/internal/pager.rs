//! `Pager<T>` (`List/Internal/Pager.cs`), and the window of items that it shares with
//! `Virtualiser<T>`.

use crate::cache::{PageRequest, PageResponse};
use crate::list::{clone_changes, remove_many, ChangeAwareList, ChangeSet, IExtendedList, ListChangeReason, PageChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// The distinct items of `all` (`Distinct()`), from `skip`, at most `take` of them.
fn distinct_window<T: Clone + PartialEq>(all: &[T], skip: usize, take: usize) -> Vec<T> {
    let mut distinct: Vec<&T> = Vec::new();
    for item in all {
        if !distinct.contains(&item) {
            distinct.push(item);
        }
    }
    distinct.into_iter().skip(skip).take(take).cloned().collect()
}

/// The items of `first` that are not in `second`, without repeats (`Except`).
fn except<T: Clone + PartialEq>(first: &[T], second: &[T]) -> Vec<T> {
    let mut result: Vec<T> = Vec::new();
    for item in first {
        if !second.contains(item) && !result.contains(item) {
            result.push(item.clone());
        }
    }
    result
}

/// Makes `window` hold `current` (the shared part of `Pager.Page` and `Virtualiser.Virtualise`):
/// removes what left, inserts what came, then repeats the moves of `change_set` that stay within
/// the window that starts at `start_index`.
pub(crate) fn update_window<T: Clone + PartialEq>(window: &mut ChangeAwareList<T>, current: &[T], change_set: Option<&ChangeSet<T>>, start_index: usize, size: usize) {
    let adds = except(current, window.items());
    let removes = except(window.items(), current);
    remove_many(window, &removes);
    for add in adds {
        let index = current.iter().position(|item| item == &add).expect("an added item is in the window");
        window.insert(index, add);
    }

    if let Some(change_set) = change_set {
        if change_set.count() != 0 {
            let moves = change_set.iter().filter(|change| change.reason() == ListChangeReason::Moved && crate::list::i_list::moved_within_range(change, start_index, start_index + size));
            for change in moves {
                // Check whether an item has moved within the same page.
                let item = change.item();
                let current_index = item.current_index().and_then(|index| index.checked_sub(start_index)).expect("the item moved to a position within the window");
                let previous_index = item.previous_index().and_then(|index| index.checked_sub(start_index)).expect("the item moved from a position within the window");
                IExtendedList::move_index(window, previous_index, current_index);
            }
        }
    }
}

struct PagerState<T> {
    all: Vec<T>,
    paged: ChangeAwareList<T>,
    parameters: PageRequest,
}

fn calculate_pages(count: usize, request: &PageRequest) -> usize {
    if request.size() >= count || request.size() == 0 {
        return 1;
    }
    let pages = count / request.size();
    let overlap = count % request.size();
    if overlap == 0 {
        pages
    } else {
        pages + 1
    }
}

fn check_parameters_and_page<T: Clone + PartialEq>(state: &mut PagerState<T>, request: &PageRequest) -> Option<PageChangeSet<T>> {
    if request.size() < 1 {
        return None;
    }
    Some(page(state, request, None))
}

fn page<T: Clone + PartialEq>(state: &mut PagerState<T>, request: &PageRequest, change_set: Option<&ChangeSet<T>>) -> PageChangeSet<T> {
    if let Some(change_set) = change_set {
        clone_changes(&mut state.all, change_set);
    }

    let pages = calculate_pages(state.all.len(), request);
    let page = request.page().min(pages);
    // Page 0 skips nothing (a negative `Skip` upstream).
    let skip = request.size() * page.saturating_sub(1);
    let current = distinct_window(&state.all, skip, request.size());

    update_window(&mut state.paged, &current, change_set, skip, request.size());

    let changed = state.paged.capture_changes();
    // The arguments in upstream's order, which passes the page as the total size and the number of
    // items as the page (`new PageResponse(paged.Count, page, all.Count, pages)`).
    PageChangeSet::new(changed, PageResponse::new(state.paged.count(), page, state.all.len(), pages))
}

/// `new Pager<T>(source, requests).Run()`.
pub(crate) fn pager<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>, requests: Rc<dyn IObservable<PageRequest>>) -> Rc<dyn IObservable<PageChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<PageChangeSet<T>>>| {
        let state = Rc::new(RefCell::new(PagerState { all: Vec::new(), paged: ChangeAwareList::new(), parameters: PageRequest::new(0, 25) }));

        let request_stream = {
            let state = state.clone();
            requests.select(move |request: PageRequest| {
                let mut state = state.borrow_mut();
                state.parameters = request;
                check_parameters_and_page(&mut state, &request)
            })
        };

        let data_changed = {
            let state = state.clone();
            source.select(move |changes: ChangeSet<T>| {
                let mut state = state.borrow_mut();
                let parameters = state.parameters;
                Some(page(&mut state, &parameters, Some(&changes)))
            })
        };

        request_stream
            .merge(data_changed)
            .where_(|changes: &Option<PageChangeSet<T>>| changes.as_ref().is_some_and(|changes| changes.count() != 0))
            .select(|changes: Option<PageChangeSet<T>>| changes.expect("filtered"))
            .subscribe(observer)
    })
}
