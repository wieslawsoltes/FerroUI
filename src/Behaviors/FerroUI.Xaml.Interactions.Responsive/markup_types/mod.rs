//! Markup metadata of the types of this crate that are not declared with their type: the lists of
//! the class setters, whose items markup adds as the children of the behaviors (their content).
//! The classes state their properties through their registered properties, as every class of the
//! framework.

use crate::{AdaptiveClassSetter, AdaptiveClassSetters, AspectRatioClassSetter, AspectRatioClassSetters};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::Ref;

ferro_markup_type!(class AdaptiveClassSetters {
    handles: [AdaptiveClassSetters, Option<AdaptiveClassSetters>],
    constructors: [() => AdaptiveClassSetters::new],
    properties: [
        Count: i32 { get: |list: &AdaptiveClassSetters| list.count() as i32 },
    ],
    methods: [
        fn Add(Ref<AdaptiveClassSetter>) =>
            |list: &AdaptiveClassSetters, item: Ref<AdaptiveClassSetter>| list.add(item)
    ],
});

ferro_markup_type!(class AspectRatioClassSetters {
    handles: [AspectRatioClassSetters, Option<AspectRatioClassSetters>],
    constructors: [() => AspectRatioClassSetters::new],
    properties: [
        Count: i32 { get: |list: &AspectRatioClassSetters| list.count() as i32 },
    ],
    methods: [
        fn Add(Ref<AspectRatioClassSetter>) =>
            |list: &AspectRatioClassSetters, item: Ref<AspectRatioClassSetter>| list.add(item)
    ],
});

/// The lists of the class setters.
const TYPES: &[&MarkupType] = &[
    <AdaptiveClassSetters as MarkupTyped>::MARKUP,
    <AspectRatioClassSetters as MarkupTyped>::MARKUP,
];

/// Adds the types of this crate to the process-wide table of types with markup metadata. Called
/// once by `register_types()`.
pub(crate) fn register() {
    MarkupType::register_all(TYPES);
    ValueTypes::register_global(register_value_types);
}

/// What the untyped value conversions must know about the types of this crate: the nullable
/// forms of the lists.
fn register_value_types() {
    ValueTypes::register_nullable::<AdaptiveClassSetters>();
    ValueTypes::register_nullable::<AspectRatioClassSetters>();
}
