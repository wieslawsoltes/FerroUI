//! `ReactiveWindow<TViewModel>` (`ReactiveWindow.cs`): a ReactiveUI `Window` that implements
//! `IViewFor<TViewModel>`.

use crate::reactive_view::ReactiveView;
use crate::ReactiveWindowBase;

/// A ReactiveUI `Window` that implements `IViewFor<TViewModel>` (`ReactiveWindow<TViewModel>`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): a typed handle over a [`ReactiveWindowBase`]
/// ([`ReactiveView`]): it dereferences to the window, gives and takes the view model as `Rc<VM>`,
/// and makes the window take only view models of type `VM` from its data context
/// (`IsValidViewModelValue`). Shared as an `Rc`, it is the window's `IViewFor<TViewModel>`
/// ([`into_view_for_of`](ReactiveView::into_view_for_of)).
pub type ReactiveWindow<VM> = ReactiveView<ReactiveWindowBase, VM>;
