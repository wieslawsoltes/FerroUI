//! Port of `StyledElement/StyledElementBehaviorOfT.cs` (`StyledElementBehavior<T>`).

use crate::ferro_object::behavior_of_t::{associated_object_as, check_associated_type};
use crate::{StyledElementBehavior, StyledElementBehaviorImpl, StyledElementBehaviorImplExt};
use ferroui_base::{ferro_class, FerroObjectImpl, ObjectType, Ref, StyledElementImpl, TypeInfo};

/// A base class for behaviors making them code compatible with older frameworks, and allow for
/// typed associated objects (`StyledElementBehavior<T>`).
///
/// Deviation (DEVIATIONS.md, Behaviors): the type argument `T` of the reference is the runtime
/// type a deriving class passes to [`construct`](Self::construct); see
/// [`BehaviorOfT`](crate::BehaviorOfT).
#[repr(C)]
pub struct StyledElementBehaviorOfT {
    base: StyledElementBehavior,
    associated_type: &'static TypeInfo,
}

ferro_class!(StyledElementBehaviorOfT: StyledElementBehavior);
ferroui_base::ferro_impl_classes!(StyledElementBehaviorOfT: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for StyledElementBehaviorOfT {
        /// Called after the behavior is attached to the associated object: checks the type of the
        /// associated object.
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            check_associated_type(this.associated_object(), this.associated_type);
        }
    }
}

impl StyledElementBehaviorOfT {
    /// Creates the class data for behaviors attached to objects of `associated_type` (the `T` of
    /// the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(associated_type: &'static TypeInfo) -> Self {
        Self { base: StyledElementBehavior::construct(), associated_type }
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
