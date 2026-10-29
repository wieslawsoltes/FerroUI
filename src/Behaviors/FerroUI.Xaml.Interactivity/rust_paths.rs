//! The public Rust paths of the registered types of this crate, by the shortest path another crate
//! names it by; the emitter of Rust source of the markup compiler reads them.

ferroui_base::ferro_rust_paths! {
    classes: [
        crate::Action,
        crate::Behavior,
        crate::BehaviorOfT,
        crate::Condition,
        crate::DisposingBehavior,
        crate::DisposingTrigger,
        crate::EventTriggerBase,
        crate::Interaction,
        crate::InteractiveBehaviorBase,
        crate::InteractiveTriggerBase,
        crate::InvokeCommandActionBase,
        crate::InvokeCommandBehaviorBase,
        crate::StyledElementAction,
        crate::StyledElementBehavior,
        crate::StyledElementBehaviorOfT,
        crate::StyledElementTrigger,
        crate::StyledElementTriggerOfT,
        crate::Trigger,
        crate::TriggerOfT,
    ],
    types: [
        crate::ActionCollection,
        crate::ActionCollectionTemplate,
        crate::BehaviorCollection,
        crate::BehaviorCollectionTemplate,
        crate::ComparisonConditionType,
        crate::ConditionCollection,
        crate::NotificationTemplate,
        crate::ObjectTemplate,
    ],
    contracts: [
    ],
    generics: [
    ],
    generic_contracts: [
    ],
}
