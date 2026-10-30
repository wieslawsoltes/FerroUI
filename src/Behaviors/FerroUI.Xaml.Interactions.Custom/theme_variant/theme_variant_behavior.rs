//! Port of `ThemeVariant/ThemeVariantBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ThemeVariantScope;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Sets the requested theme variant of the associated [`ThemeVariantScope`] while it is in the
/// visual tree, and restores the previous one when it leaves it.
#[repr(C)]
pub struct ThemeVariantBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(ThemeVariantBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ThemeVariantBehavior { new: ThemeVariantBehavior::new });
ferroui_base::ferro_impl_classes!(
    ThemeVariantBehavior: StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ThemeVariantBehavior {
        /// Identifies the `ThemeVariant` property.
        pub fn theme_variant_property() -> StyledProperty<Option<ThemeVariant>> {
            FerroProperty::register::<ThemeVariantBehavior, _>("ThemeVariant", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ThemeVariantBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<ThemeVariantScope>() else {
                return DisposableAction::empty();
            };

            let old = associated_object.requested_theme_variant();
            associated_object
                .set_current_value(ThemeVariantScope::requested_theme_variant_property(), this.theme_variant());

            let behavior = this.to_ref().downgrade();
            DisposableAction::create(move || {
                if let Some(associated_object) =
                    behavior.upgrade().and_then(|behavior| behavior.associated_object_as::<ThemeVariantScope>())
                {
                    associated_object.set_current_value(ThemeVariantScope::requested_theme_variant_property(), old.clone());
                }
            })
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ThemeVariantBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::theme_variant_property().as_property() {
                if let Some(associated_object) = this.associated_object_as::<ThemeVariantScope>() {
                    associated_object.set_current_value(
                        ThemeVariantScope::requested_theme_variant_property(),
                        change.get_new_value::<Option<ThemeVariant>>(),
                    );
                }
            }
        }
    }
}

impl ThemeVariantBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<ThemeVariantScope as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the theme variant to assign. This is a property of the object model.
    pub fn theme_variant(&self) -> Option<ThemeVariant> {
        self.get_value(Self::theme_variant_property())
    }

    pub fn set_theme_variant(&self, value: Option<ThemeVariant>) {
        self.set_value(Self::theme_variant_property(), value)
    }
}
