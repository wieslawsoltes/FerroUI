//! Port of `Icon/PathIconDataBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::media::Geometry;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Nullable, Ref,
    StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::PathIcon;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// Sets the [`PathIcon::data`] when the associated icon is attached to the visual tree; the data
/// the icon had is restored when it leaves the visual tree.
#[repr(C)]
pub struct PathIconDataBehavior {
    base: AttachedToVisualTreeBehavior,
    old_data: RefCell<Option<Ref<Geometry>>>,
}

ferro_class!(PathIconDataBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(PathIconDataBehavior { new: PathIconDataBehavior::new });
ferroui_base::ferro_impl_classes!(
    PathIconDataBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl PathIconDataBehavior {
        /// Identifies the `Data` property.
        pub fn data_property() -> StyledProperty<Option<Ref<Geometry>>> {
            FerroProperty::register::<PathIconDataBehavior, _>("Data", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for PathIconDataBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<PathIcon>() else {
                return DisposableAction::empty();
            };

            *this.old_data.borrow_mut() = associated_object.data();
            if let Some(data) = this.data() {
                associated_object.set_data(data);
            }

            let behavior = this.to_ref().downgrade();
            DisposableAction::create(move || {
                if let Some(behavior) = behavior.upgrade() {
                    if let Some(associated_object) = behavior.associated_object_as::<PathIcon>() {
                        let old_data = behavior.old_data.borrow().clone();
                        associated_object.set_data(old_data);
                    }
                }
            })
        }
    }
}

impl PathIconDataBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<PathIcon as StaticType>::TYPE), old_data: RefCell::new(None) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the geometry used for the icon. This is a property of the object model.
    pub fn data(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::data_property())
    }

    pub fn set_data(&self, value: impl Into<Nullable<Geometry>>) {
        self.set_value(Self::data_property(), value.into().0)
    }
}
