//! Port of `Actions/AddClassAction.cs`.

use super::styled_element_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElement, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Adds a style class to the [`styled_element`](Self::styled_element), or to the sender when it
/// is not set.
#[repr(C)]
pub struct AddClassAction {
    base: StyledElementAction,
}

ferro_class!(AddClassAction: StyledElementAction);
ferro_class_info!(AddClassAction {
    new: AddClassAction::new,
    markup: {
        property_attributes: [
            StyledElement: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(AddClassAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl AddClassAction {
        /// Identifies the `ClassName` property.
        pub fn class_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<AddClassAction, _>("ClassName", None)
        }

        /// Identifies the `StyledElement` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn styled_element_property() -> StyledProperty<Option<ElementRef<StyledElement>>> {
            ValueTypes::register_element_ref::<StyledElement>();
            FerroProperty::register::<AddClassAction, _>("StyledElement", None)
        }

        /// Identifies the `RemoveIfExists` property.
        pub fn remove_if_exists_property() -> StyledProperty<bool> {
            FerroProperty::register::<AddClassAction, _>("RemoveIfExists", false)
        }
    }
}

impl StyledElementActionImpl for AddClassAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let target = if this.get_value(Self::styled_element_property()).is_some() {
            this.styled_element()
        } else {
            styled_element_of(sender)
        };
        let class_name = this.class_name().filter(|class_name| !class_name.is_empty());
        let (Some(target), Some(class_name)) = (target, class_name) else {
            return Some(Rc::new(false));
        };

        if this.remove_if_exists() && target.classes().contains(&class_name) {
            target.classes().remove(&class_name);
        }

        target.classes().add(&class_name);

        Some(Rc::new(true))
    }
}

impl AddClassAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the class name to add. This is a property of the object model.
    pub fn class_name(&self) -> Option<String> {
        self.get_value(Self::class_name_property())
    }

    pub fn set_class_name(&self, value: Option<String>) {
        self.set_value(Self::class_name_property(), value)
    }

    /// Gets or sets the target styled element that class name will be added to. If not set, the
    /// sender is used. Markup resolves a name given for it to the named element.
    pub fn styled_element(&self) -> Option<Ref<StyledElement>> {
        ElementRef::resolve(&self.get_value(Self::styled_element_property()))
    }

    pub fn set_styled_element(&self, value: impl Into<Nullable<StyledElement>>) {
        self.set_value(Self::styled_element_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets whether the class is removed before it is added (so that styles are applied
    /// again). This is a property of the object model.
    pub fn remove_if_exists(&self) -> bool {
        self.get_value(Self::remove_if_exists_property())
    }

    pub fn set_remove_if_exists(&self, value: bool) {
        self.set_value(Self::remove_if_exists_property(), value)
    }
}
