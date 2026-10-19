//! `RoutingStateMixins` (`ReactiveUI.Shared/Routing/RoutingStateMixins.cs`).

use super::routing_state::RoutableViewModel;
use crate::{IRoutableViewModel, RoutingState};
use std::any::Any;
use std::rc::Rc;

/// Queries of the navigation stack of a [`RoutingState`] (`RoutingStateMixins`).
pub trait RoutingStateMixins {
    /// The view model of the type `T` nearest the top of the stack, or `None` when the stack has
    /// none (`FindViewModelInStack<T>()`).
    fn find_view_model_in_stack<T: IRoutableViewModel>(&self) -> Option<Rc<T>>;

    /// The view model on top of the stack, or `None` when it is empty (`GetCurrentViewModel()`).
    fn get_current_view_model(&self) -> Option<RoutableViewModel>;
}

impl RoutingStateMixins for RoutingState {
    fn find_view_model_in_stack<T: IRoutableViewModel>(&self) -> Option<Rc<T>> {
        let stack = self.navigation_stack().to_vec();
        stack.into_iter().rev().find_map(|view_model| {
            let view_model: Rc<dyn Any> = view_model;
            view_model.downcast::<T>().ok()
        })
    }

    fn get_current_view_model(&self) -> Option<RoutableViewModel> {
        let stack = self.navigation_stack();
        let count = stack.count();
        if count > 0 {
            Some(stack.get(count - 1))
        } else {
            None
        }
    }
}
