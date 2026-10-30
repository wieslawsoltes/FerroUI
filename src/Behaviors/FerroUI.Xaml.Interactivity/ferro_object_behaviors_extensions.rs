//! Port of the extensions of the reference that surface `Interaction.BehaviorsProperty` on the root
//! object class.

use crate::{BehaviorCollection, Interaction};
use ferroui_base::{AttachedProperty, FerroObject};

/// Surfaces [`Interaction::behaviors_property`] directly on [`FerroObject`]: the static members
/// `FerroObject::behaviors_property()`, `FerroObject::get_behaviors(..)` and
/// `FerroObject::set_behaviors(..)` when this trait is in scope.
///
/// The first use of the members adds the root object class as an owner of the attached
/// property (the static constructor of the reference), so that markup can name it with the
/// inherited syntax of a property element (`<Border.Behaviors>`).
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference declares the members as extension
/// members of the object class (`extension(FerroObject)`); a Rust trait implemented for the
/// root class gives the same calls.
pub trait FerroObjectBehaviorsExtensions {
    /// Exposes the behaviors attached property for consumers targeting [`FerroObject`].
    fn behaviors_property() -> &'static AttachedProperty<Option<BehaviorCollection>>;

    /// Delegates to [`Interaction::get_behaviors`].
    fn get_behaviors(obj: &FerroObject) -> BehaviorCollection;

    /// Delegates to [`Interaction::set_behaviors`].
    fn set_behaviors(obj: &FerroObject, value: Option<BehaviorCollection>);
}

/// The static constructor: `Interaction.BehaviorsProperty.AddOwner<FerroObject>()`, once per
/// thread (property definitions belong to their thread).
fn ensure_owner() {
    ferroui_base::ferro_thread_local! {
        static ADDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if !ADDED.replace(true) {
        let _ = Interaction::behaviors_property().add_owner::<FerroObject>();
    }
}

impl FerroObjectBehaviorsExtensions for FerroObject {
    fn behaviors_property() -> &'static AttachedProperty<Option<BehaviorCollection>> {
        ensure_owner();
        Interaction::behaviors_property()
    }

    fn get_behaviors(obj: &FerroObject) -> BehaviorCollection {
        ensure_owner();
        Interaction::get_behaviors(obj)
    }

    fn set_behaviors(obj: &FerroObject, value: Option<BehaviorCollection>) {
        ensure_owner();
        Interaction::set_behaviors(obj, value)
    }
}
