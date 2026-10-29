//! Port of `Icon/SetPathIconDataAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::Geometry;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::PathIcon;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Changes the [`PathIcon::data`] of a target icon when executed.
#[repr(C)]
pub struct SetPathIconDataAction {
    base: StyledElementAction,
}

ferro_class!(SetPathIconDataAction: StyledElementAction);
ferro_class_info!(SetPathIconDataAction {
    new: SetPathIconDataAction::new,
    markup: {
        property_attributes: [
            PathIcon: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SetPathIconDataAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetPathIconDataAction {
        /// Identifies the `Data` property.
        pub fn data_property() -> StyledProperty<Option<Ref<Geometry>>> {
            FerroProperty::register::<SetPathIconDataAction, _>("Data", None)
        }

        /// Identifies the `PathIcon` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the icon, so the value is
        /// an element reference.
        pub fn path_icon_property() -> StyledProperty<Option<ElementRef<PathIcon>>> {
            ValueTypes::register_element_ref::<PathIcon>();
            FerroProperty::register::<SetPathIconDataAction, _>("PathIcon", None)
        }
    }
}

impl StyledElementActionImpl for SetPathIconDataAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let target =
            this.path_icon().or_else(|| sender.and_then(object_of).and_then(|sender| sender.cast::<PathIcon>()));
        let Some(target) = target else {
            return Some(Rc::new(false));
        };

        target.set_data(this.data());
        Some(Rc::new(true))
    }
}

impl SetPathIconDataAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the geometry to apply. This is a property of the object model.
    pub fn data(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::data_property())
    }

    pub fn set_data(&self, value: impl Into<Nullable<Geometry>>) {
        self.set_value(Self::data_property(), value.into().0)
    }

    /// Gets or sets the target icon. This is a property of the object model. Markup resolves a
    /// name given for it to the named element.
    pub fn path_icon(&self) -> Option<Ref<PathIcon>> {
        ElementRef::resolve(&self.get_value(Self::path_icon_property()))
    }

    pub fn set_path_icon(&self, value: impl Into<Nullable<PathIcon>>) {
        self.set_value(Self::path_icon_property(), ElementRef::from_nullable(value.into().0))
    }
}
