//! ferroui-xaml-interactivity
//!
//! The object model of the behaviors of FerroUI, ported from the `Xaml.Behaviors.Interactivity`
//! project of Xaml.Behaviors (see `NOTICE.md`):
//!
//! - the attached property [`Interaction::behaviors_property`] and its [`BehaviorCollection`],
//!   which attaches its behaviors to the host and replays the lifecycle of the host to them
//!   (initialized, attached to the logical tree, attached to the visual tree, loaded), phase by
//!   phase across the behaviors, including the behaviors added while it does so;
//! - the contracts [`IBehavior`], [`ITrigger`], [`IAction`], [`IReversibleAction`] and
//!   [`IBehaviorEventsHandler`];
//! - the behaviors, triggers and actions that are plain objects ([`Behavior`], [`Trigger`],
//!   [`Action`]) and the ones that are styled elements and join the logical tree of their host
//!   ([`StyledElementBehavior`], [`StyledElementTrigger`], [`StyledElementAction`]), with their
//!   typed forms ([`BehaviorOfT`], [`StyledElementBehaviorOfT`], ..);
//! - the base classes of event triggers, command behaviors and actions, and disposing behaviors;
//! - the templates that let a style set behaviors and actions ([`BehaviorCollectionTemplate`],
//!   [`ActionCollectionTemplate`]) or other objects ([`ObjectTemplate`], [`NotificationTemplate`]);
//! - the helpers the other behavior crates use: properties and methods by name, conversions and
//!   comparisons of untyped values, command state, reversible property changes.
//!
//! The namespace of the crate is `FerroUI.Xaml.Interactivity`, mapped into the default XML
//! namespace of the framework: documents name its types without a prefix once
//! [`register_types`] ran.

mod actions;
mod behaviors;
mod collections;
mod contract;
mod core;
mod disposable_action;
mod events;
mod ferro_object;
mod ferro_object_behaviors_extensions;
pub mod helpers;
mod interaction;
mod markup_types;
mod register_types;
mod rust_paths;
mod styled_element;
mod templates;
mod triggers;

pub use crate::actions::InvokeCommandActionBase;
pub use crate::behaviors::{
    DisposingBehavior, DisposingBehaviorImpl, DisposingBehaviorImplExt, DisposingBehaviorVTable, InteractiveBehaviorBase,
    InvokeCommandBehaviorBase,
};
pub use crate::collections::{ActionCollection, BehaviorCollection};
pub use crate::contract::{
    ComparisonConditionType, IAction, IBehavior, IBehaviorEventsHandler, IReversibleAction, ITrigger,
};
pub use crate::core::{Condition, ConditionCollection};
pub use crate::disposable_action::DisposableAction;
pub use crate::events::{
    AddEventHandlerCallback, AddEventHandlerRegistry, EventArgsValue, EventHandler, FuncAddEventHandler, IAddEventHandler,
};
pub use crate::ferro_object::{
    Action, ActionImpl, ActionImplExt, ActionVTable, Behavior, BehaviorImpl, BehaviorImplExt, BehaviorOfT, BehaviorVTable,
    Trigger, TriggerOfT,
};
pub use crate::ferro_object_behaviors_extensions::FerroObjectBehaviorsExtensions;
pub use crate::helpers::{CommandCanExecuteObserver, ReversiblePropertyChange};
pub use crate::interaction::Interaction;
pub use crate::register_types::{register_types, ASSEMBLY};
pub use crate::styled_element::{
    IActionLogicalTreeLifecycle, StyledElementAction, StyledElementActionImpl, StyledElementActionImplExt,
    StyledElementActionVTable, StyledElementBehavior, StyledElementBehaviorImpl, StyledElementBehaviorImplExt,
    StyledElementBehaviorOfT, StyledElementBehaviorVTable, StyledElementTrigger, StyledElementTriggerOfT,
};
pub use crate::templates::{ActionCollectionTemplate, BehaviorCollectionTemplate, NotificationTemplate, ObjectTemplate};
pub use crate::triggers::{
    DisposingTrigger, DisposingTriggerImpl, DisposingTriggerImplExt, DisposingTriggerVTable, EventTriggerBase,
    EventTriggerBaseImpl, EventTriggerBaseImplExt, EventTriggerBaseVTable, InteractiveTriggerBase,
};

#[cfg(test)]
mod tests;
