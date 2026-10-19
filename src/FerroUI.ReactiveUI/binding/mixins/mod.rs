//! `ReactiveUI.Binding/Mixins`: the property observation of an object (`WhenAnyValue`, `WhenAny`,
//! `WhenAnyObservable`, `WhenChanged`, `WhenChanging`), `ObservedProperty` and `ToProperty`; the
//! bindings (`Bind`, `OneWayBind`, `BindTo`, `BindCommand`, `BindInteraction`, `InvokeCommand`),
//! `ReactiveSchedulerExtensions` and the builder mixins.
//!
//! Not ported: the `*.Unsafe.cs` overloads (expression trees and reflection) and `WhenAnyDynamic`
//! (its arguments are untyped expressions read by reflection; `observable_for_property` has the
//! `is_distinct` option it adds).

mod observable_chain_helpers;
mod observed_property;
mod reactive_ui_binding_extensions;
mod reactive_ui_binding_extensions_invoke_command;
mod to_property;
mod when_any;
mod when_any_observable;
mod when_any_observable_extended;
mod when_any_value;
mod when_any_value_selectors;
mod when_changed;
mod when_changed_selectors;
mod when_changing;
mod when_changing_selectors;

pub use observable_chain_helpers::{switch_latest, ObservableLeaf};
pub use observed_property::ObservedProperty;
pub use reactive_ui_binding_extensions::ReactiveUIBindingExtensions;
pub use reactive_ui_binding_extensions_invoke_command::InvokeCommandExt;
pub use to_property::ToPropertyMixins;
pub use when_any_observable::WhenAnyObservablePaths;
pub use when_any_observable_extended::WhenAnyObservableCombinePaths;
pub use when_any_value::WhenAnyPaths;
mod reactive_scheduler_extensions;
mod reactive_ui_binding_extensions_bind_to;
mod reactive_ui_binding_extensions_binding;
pub(crate) use reactive_scheduler_extensions::{schedule_latest_write, schedule_write};
pub use reactive_ui_binding_extensions_bind_to::BindToMixins;
pub use reactive_ui_binding_extensions_binding::{PropertyBindingMixins, SourceBindingMixins};
mod builder_mixins;
mod reactive_ui_binding_extensions_bind_command;
mod reactive_ui_binding_extensions_bind_interaction;
pub use builder_mixins::BuilderMixins;
pub use reactive_ui_binding_extensions_bind_command::{BindCommandMixins, CommandValue, ControlValue, ToEvent};
pub use reactive_ui_binding_extensions_bind_interaction::{BindInteractionMixins, InteractionContextOf, InteractionValue};
