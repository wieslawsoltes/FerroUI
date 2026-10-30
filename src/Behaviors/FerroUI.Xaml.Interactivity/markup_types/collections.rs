//! Markup metadata of the collections (`Collections/ActionCollection.cs`,
//! `Collections/BehaviorCollection.cs`, `Core/ConditionCollection.cs`): lists whose items markup
//! adds as the children of the collection (or of the property element of a property that holds
//! one).

use crate::{ActionCollection, BehaviorCollection, Condition, ConditionCollection};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{FerroObject, Ref};

ferro_markup_type!(class ActionCollection {
    handles: [ActionCollection, Option<ActionCollection>],
    constructors: [() => ActionCollection::new],
    properties: [
        Count: i32 { get: |list: &ActionCollection| list.count() as i32 },
    ],
    methods: [fn Add(Ref<FerroObject>) => |list: &ActionCollection, item: Ref<FerroObject>| list.add(item)],
});

ferro_markup_type!(class BehaviorCollection {
    handles: [BehaviorCollection, Option<BehaviorCollection>],
    constructors: [() => BehaviorCollection::new],
    properties: [
        Count: i32 { get: |list: &BehaviorCollection| list.count() as i32 },
        AssociatedObject: Option<Ref<FerroObject>> { get: |list: &BehaviorCollection| list.associated_object() },
    ],
    methods: [fn Add(Ref<FerroObject>) => |list: &BehaviorCollection, item: Ref<FerroObject>| list.add(item)],
});

ferro_markup_type!(class ConditionCollection {
    handles: [ConditionCollection, Option<ConditionCollection>],
    constructors: [() => ConditionCollection::new],
    properties: [
        Count: i32 { get: |list: &ConditionCollection| list.count() as i32 },
    ],
    methods: [fn Add(Ref<Condition>) => |list: &ConditionCollection, item: Ref<Condition>| list.add(item)],
});

/// The collections.
pub(super) const TYPES: &[&MarkupType] = &[
    <ActionCollection as MarkupTyped>::MARKUP,
    <BehaviorCollection as MarkupTyped>::MARKUP,
    <ConditionCollection as MarkupTyped>::MARKUP,
];

pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<ActionCollection>();
    ValueTypes::register_nullable::<BehaviorCollection>();
    ValueTypes::register_nullable::<ConditionCollection>();
}
