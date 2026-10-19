//! `ReactiveUI.Binding/ObservableForProperty`: the observers of properties chosen at run time and
//! the observation of a property path, step by step.
//!
//! `ChainLinkReader` (the read of one member of an expression by reflection) and `NotifyFactoryKey`
//! (the key of the memo of the resolved observers) are not ported: a step of a path reads its own
//! value ([`PathStep::get`](ferroui_system::component_model::PathStep)), and the observer is chosen
//! on each object, which is not memoized.

mod expression_chain_parameters;
mod expression_chain_sink;
mod inpc_observable_for_property;
mod observable_for_property_sink;
mod observed_changed_mixins;
mod observed_property_key;
mod poco_observable_for_property;
mod reactive_notify_property_changed_mixins;

pub use expression_chain_parameters::{ExpressionChainParameters, ValueComparer};
pub use expression_chain_sink::ExpressionChainSink;
pub use inpc_observable_for_property::INPCObservableForProperty;
pub use observable_for_property_sink::ObservableForPropertySink;
pub use observed_changed_mixins::{ObservedChangedMixins, ObservedChangedStreamMixins};
pub use observed_property_key::ObservedPropertyKey;
pub use poco_observable_for_property::POCOObservableForProperty;
pub use reactive_notify_property_changed_mixins::{
    find_observer_for_property, notify_for_property, observers_for_property, ReactiveNotifyPropertyChangedMixins,
};
