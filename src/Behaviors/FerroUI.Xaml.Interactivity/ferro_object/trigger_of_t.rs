//! Port of `FerroObject/TriggerOfT.cs` (`Trigger<T>`).

use crate::ferro_object::behavior_of_t::{associated_object_as, check_associated_type};
use crate::{BehaviorImpl, BehaviorImplExt, Trigger};
use ferroui_base::{ferro_class, FerroObjectImpl, ObjectType, Ref, TypeInfo};

/// A base class for behaviors, implementing the basic plumbing of [`ITrigger`](crate::ITrigger),
/// with a typed associated object (`Trigger<T>`).
///
/// Deviation (DEVIATIONS.md, Behaviors): the type argument `T` of the reference is the runtime
/// type a deriving class passes to [`construct`](Self::construct); see
/// [`BehaviorOfT`](crate::BehaviorOfT).
#[repr(C)]
pub struct TriggerOfT {
    base: Trigger,
    associated_type: &'static TypeInfo,
}

ferro_class!(TriggerOfT: Trigger);
ferroui_base::ferro_impl_classes!(TriggerOfT: FerroObjectImpl);

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for TriggerOfT {
        /// Called after the behavior is attached to the associated object: checks the type of the
        /// associated object.
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            check_associated_type(this.associated_object(), this.associated_type);
        }
    }
}

impl TriggerOfT {
    /// Creates the class data for triggers attached to objects of `associated_type` (the `T` of
    /// the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        Self { base: Trigger::construct(), associated_type }
    }

    /// The type of the objects the trigger attaches to (the `T` of the reference).
    pub fn associated_type(&self) -> &'static TypeInfo {
        self.associated_type
    }

    /// Gets the object to which this behavior is attached, as `T`.
    pub fn associated_object_as<T: ObjectType>(&self) -> Option<Ref<T>> {
        associated_object_as::<T>(self.associated_object())
    }
}
