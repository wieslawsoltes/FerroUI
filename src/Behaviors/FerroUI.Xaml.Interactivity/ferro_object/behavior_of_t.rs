//! Port of `FerroObject/BehaviorOfT.cs` (`Behavior<T>`).

use crate::{Behavior, BehaviorImpl, BehaviorImplExt};
use ferroui_base::{ferro_class, FerroObject, FerroObjectImpl, ObjectType, Ref, TypeInfo};

/// A base class for behaviors making them code compatible with older frameworks, and allow for
/// typed associated objects (`Behavior<T>`).
///
/// Deviation (DEVIATIONS.md, Behaviors): classes of the class model are not generic. The type
/// argument `T` of the reference is the runtime type a deriving class passes to
/// [`construct`](Self::construct); attaching the behavior to an object of another type panics as
/// the reference throws, and [`associated_object_as`](Self::associated_object_as) is the typed
/// `AssociatedObject` of the reference.
#[repr(C)]
pub struct BehaviorOfT {
    base: Behavior,
    associated_type: &'static TypeInfo,
}

ferro_class!(BehaviorOfT: Behavior);
ferroui_base::ferro_impl_classes!(BehaviorOfT: FerroObjectImpl);

ferroui_base::ferro_overrides! {
    impl BehaviorImpl for BehaviorOfT {
        /// Called after the behavior is attached to the associated object: checks the type of the
        /// associated object.
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            check_associated_type(this.associated_object(), this.associated_type);
        }
    }
}

/// Panics as the reference throws when the associated object is not of the expected type
/// (`AssociatedObject is null && base.AssociatedObject is not null`).
pub(crate) fn check_associated_type(associated_object: Option<Ref<FerroObject>>, associated_type: &'static TypeInfo) {
    if let Some(associated_object) = associated_object {
        let actual_type = associated_object.get_type();
        if !associated_type.is_assignable_from(actual_type) {
            panic!(
                "AssociatedObject is of type {} but should be of type {}.",
                actual_type.full_name(),
                associated_type.full_name()
            );
        }
    }
}

/// The associated object as `T`, if it is one.
pub(crate) fn associated_object_as<T: ObjectType>(associated_object: Option<Ref<FerroObject>>) -> Option<Ref<T>> {
    associated_object.and_then(|associated_object| associated_object.cast::<T>())
}

impl BehaviorOfT {
    /// Creates the class data for behaviors attached to objects of `associated_type` (the `T` of
    /// the reference); see [`FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        Self { base: Behavior::construct(), associated_type }
    }

    /// The type of the objects the behavior attaches to (the `T` of the reference).
    pub fn associated_type(&self) -> &'static TypeInfo {
        self.associated_type
    }

    /// Gets the object to which this behavior is attached, as `T`.
    pub fn associated_object_as<T: ObjectType>(&self) -> Option<Ref<T>> {
        associated_object_as::<T>(self.associated_object())
    }
}
