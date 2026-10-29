//! Markup metadata of `Interaction.cs`: the accessors of the attached property
//! `Interaction.Behaviors`.
//!
//! The getter is the one of the reference, which creates the collection when the object has none:
//! markup's property element syntax (`<Interaction.Behaviors>`) adds the behaviors to the
//! collection the getter returns. Declaring it replaces the plain accessor the type system
//! projects for the registered property, which only reads the value.

use crate::{BehaviorCollection, Interaction};
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{FerroObject, Ref};

ferro_markup_type!(static Interaction {
    type_info: Interaction,
    methods: [
        static fn GetBehaviors(Ref<FerroObject>) -> BehaviorCollection => |obj: Ref<FerroObject>| Interaction::get_behaviors(&obj),
        static fn SetBehaviors(Ref<FerroObject>, Option<BehaviorCollection>) =>
            |obj: Ref<FerroObject>, value: Option<BehaviorCollection>| Interaction::set_behaviors(&obj, value),
    ],
});

/// The static types.
pub(super) const TYPES: &[&MarkupType] = &[<Interaction as MarkupTyped>::MARKUP];
