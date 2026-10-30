//! Port of `ContextDialogs/ShowContextDialogAction.cs`.

use crate::ContextDialogBehavior;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Opens a [`ContextDialogBehavior`] when executed.
#[repr(C)]
pub struct ShowContextDialogAction {
    base: StyledElementAction,
}

ferro_class!(ShowContextDialogAction: StyledElementAction);
ferro_class_info!(ShowContextDialogAction {
    new: ShowContextDialogAction::new,
    markup: {
        property_attributes: [
            TargetDialog: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ShowContextDialogAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ShowContextDialogAction {
        /// Identifies the `TargetDialog` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the behavior, so the value
        /// is an element reference.
        pub fn target_dialog_property() -> StyledProperty<Option<ElementRef<ContextDialogBehavior>>> {
            ValueTypes::register_element_ref::<ContextDialogBehavior>();
            FerroProperty::register::<ShowContextDialogAction, _>("TargetDialog", None)
        }
    }
}

impl StyledElementActionImpl for ShowContextDialogAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let Some(dialog) = this.target_dialog() else {
            return Some(Rc::new(false));
        };

        dialog.set_current_value(ContextDialogBehavior::is_open_property(), true);
        Some(Rc::new(true))
    }
}

impl ShowContextDialogAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target dialog behavior. This is a property of the object model; markup
    /// resolves a name given for it to the named behavior.
    pub fn target_dialog(&self) -> Option<Ref<ContextDialogBehavior>> {
        ElementRef::resolve(&self.get_value(Self::target_dialog_property()))
    }

    pub fn set_target_dialog(&self, value: impl Into<Nullable<ContextDialogBehavior>>) {
        self.set_value(Self::target_dialog_property(), ElementRef::from_nullable(value.into().0))
    }
}
