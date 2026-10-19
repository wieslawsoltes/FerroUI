//! `Virtualiser<T>` (`List/Internal/Virtualiser.cs`).

use super::pager::update_window;
use crate::cache::{VirtualRequest, VirtualResponse};
use crate::list::{clone_changes, ChangeAwareList, ChangeSet, VirtualChangeSet};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

struct VirtualiserState<T> {
    all: Vec<T>,
    virtualised: ChangeAwareList<T>,
    parameters: VirtualRequest,
}

fn check_params_and_virtualise<T: Clone + PartialEq>(state: &mut VirtualiserState<T>, request: &VirtualRequest) -> Option<ChangeSet<T>> {
    if request.size() < 1 {
        return None;
    }
    Some(virtualise(state, request, None))
}

fn virtualise<T: Clone + PartialEq>(state: &mut VirtualiserState<T>, request: &VirtualRequest, change_set: Option<&ChangeSet<T>>) -> ChangeSet<T> {
    if let Some(change_set) = change_set {
        clone_changes(&mut state.all, change_set);
    }

    let mut distinct: Vec<&T> = Vec::new();
    for item in &state.all {
        if !distinct.contains(&item) {
            distinct.push(item);
        }
    }
    let current: Vec<T> = distinct.into_iter().skip(request.start_index()).take(request.size()).cloned().collect();

    update_window(&mut state.virtualised, &current, change_set, request.start_index(), request.size());

    state.virtualised.capture_changes()
}

/// `new Virtualiser<T>(source, requests).Run()`.
pub(crate) fn virtualiser<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>, requests: Rc<dyn IObservable<VirtualRequest>>) -> Rc<dyn IObservable<VirtualChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<VirtualChangeSet<T>>>| {
        let state = Rc::new(RefCell::new(VirtualiserState { all: Vec::new(), virtualised: ChangeAwareList::new(), parameters: VirtualRequest::new(0, 25) }));

        let request_stream = {
            let state = state.clone();
            requests.select(move |request: VirtualRequest| {
                let mut state = state.borrow_mut();
                state.parameters = request;
                check_params_and_virtualise(&mut state, &request)
            })
        };

        let data_changed = {
            let state = state.clone();
            source.select(move |changes: ChangeSet<T>| {
                let mut state = state.borrow_mut();
                let parameters = state.parameters;
                Some(virtualise(&mut state, &parameters, Some(&changes)))
            })
        };

        request_stream
            .merge(data_changed)
            .where_(|changes: &Option<ChangeSet<T>>| changes.as_ref().is_some_and(|changes| changes.count() != 0))
            .select(move |changes: Option<ChangeSet<T>>| {
                let state = state.borrow();
                VirtualChangeSet::new(changes.expect("filtered"), VirtualResponse::new(state.virtualised.count(), state.parameters.start_index(), state.all.len()))
            })
            .subscribe(observer)
    })
}
