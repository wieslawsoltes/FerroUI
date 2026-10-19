//! `ReactiveUI.Shared/Routing` and `ReactiveUI.Core/Routing`: the navigation stack of a screen and
//! the message bus.
//!
//! - [`RoutingState`]: the stack of the view models of an [`IScreen`](crate::IScreen), with the
//!   commands that navigate it and the observables of the navigation; [`RoutingStateMixins`]
//!   queries it, and [`RoutableViewModelMixins`] tells a view model when it gains and loses the
//!   focus.
//! - [`MessageBus`]: messages between objects that do not know each other, by type and contract,
//!   with the typed operations of [`IMessageBusExt`].

mod message_bus;
mod not_a_weak_reference;
mod routable_view_model_mixins;
mod routing_state;
mod routing_state_mixins;
mod skip_first_observer;

pub use message_bus::{IMessageBusExt, MessageBus};
pub use not_a_weak_reference::NotAWeakReference;
pub use routable_view_model_mixins::RoutableViewModelMixins;
pub use routing_state::{NavigateBackCommand, NavigateCommand, NavigationStack, NavigationStackSnapshot, RoutableViewModel, RoutingState};
pub use routing_state_mixins::RoutingStateMixins;
pub use skip_first_observer::SkipFirstObserver;
