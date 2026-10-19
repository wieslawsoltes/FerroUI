//! `ReactiveUserControl<TViewModel>` (`ReactiveUserControl.cs`): a ReactiveUI `UserControl` that
//! implements `IViewFor<TViewModel>`.

use crate::reactive_view::ReactiveView;
use crate::ReactiveUserControlBase;

/// A ReactiveUI `UserControl` that implements `IViewFor<TViewModel>`
/// (`ReactiveUserControl<TViewModel>`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): a typed handle over a
/// [`ReactiveUserControlBase`] ([`ReactiveView`]): it dereferences to the control, gives and
/// takes the view model as `Rc<VM>`, and makes the control take only view models of type `VM` from
/// its data context (`IsValidViewModelValue`). Shared as an `Rc`, it is the control's
/// `IViewFor<TViewModel>` ([`into_view_for_of`](ReactiveView::into_view_for_of)). A class derived
/// from the base (one of markup) is typed with [`from_control`](ReactiveView::from_control).
///
/// ```ignore
/// let view = ReactiveUserControl::<LoginViewModel>::new();
/// view.set_view_model(Some(view_model.clone()));
/// assert!(view.data_context().is_some());
/// ```
pub type ReactiveUserControl<VM> = ReactiveView<ReactiveUserControlBase, VM>;
