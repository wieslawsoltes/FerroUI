//! ferroui-xaml-interactions-reactiveui
//!
//! The ReactiveUI behaviors of FerroUI, ported from the `Xaml.Behaviors.Interactions.ReactiveUI`
//! project of Xaml.Behaviors (see `NOTICE.md`):
//!
//! - actions that navigate a ReactiveUI router ([`ferroui_reactiveui::RoutingState`]): to a view
//!   model ([`NavigateAction`]), to a view model with a reset of the stack
//!   ([`NavigateAndReset`]), back ([`NavigateBackAction`]), and the reset of the stack alone
//!   ([`ClearNavigationStackAction`]);
//! - actions that navigate to a view model of a type, given as the parameter or resolved from the
//!   service locator ([`NavigateToAction`], [`NavigateToAndResetAction`], with the type as a
//!   [`RoutableViewModelType`]);
//! - the trigger that executes its actions when a ReactiveUI interaction is handled
//!   ([`InteractionTriggerBehavior`], with the interaction as an [`AnyInteraction`]).
//!
//! The namespace of the crate is `FerroUI.Xaml.Interactions.ReactiveUI`, mapped into the default
//! XML namespace of the framework: documents name its types without a prefix once
//! [`register_types`] ran.

mod clear_navigation_stack_action;
mod interaction_trigger_behavior;
mod markup_types;
mod navigate_action;
mod navigate_and_reset;
mod navigate_back_action;
mod navigate_to_action;
mod navigate_to_and_reset_action;
mod register_types;
mod rust_paths;
mod view_model_parameter;

pub use crate::clear_navigation_stack_action::ClearNavigationStackAction;
pub use crate::interaction_trigger_behavior::{AnyInteraction, InteractionTriggerBehavior};
pub use crate::navigate_action::NavigateAction;
pub use crate::navigate_and_reset::NavigateAndReset;
pub use crate::navigate_back_action::NavigateBackAction;
pub use crate::navigate_to_action::{
    NavigateToAction, NavigateToActionImpl, NavigateToActionImplExt, NavigateToActionVTable, RoutableViewModelType,
};
pub use crate::navigate_to_and_reset_action::{
    NavigateToAndResetAction, NavigateToAndResetActionImpl, NavigateToAndResetActionImplExt,
    NavigateToAndResetActionVTable,
};
pub use crate::register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
