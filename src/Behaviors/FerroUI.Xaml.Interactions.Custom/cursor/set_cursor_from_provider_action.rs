//! Port of `Cursor/SetCursorFromProviderAction.cs`.

use crate::ICursorProvider;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElement;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Sets the cursor on a target control using an [`ICursorProvider`].
#[repr(C)]
pub struct SetCursorFromProviderAction {
    base: StyledElementAction,
}

ferro_class!(SetCursorFromProviderAction: StyledElementAction);
ferro_class_info!(SetCursorFromProviderAction {
    new: SetCursorFromProviderAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SetCursorFromProviderAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetCursorFromProviderAction {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the control, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<InputElement>>> {
            ValueTypes::register_element_ref::<InputElement>();
            FerroProperty::register::<SetCursorFromProviderAction, _>("TargetControl", None)
        }

        /// Identifies the `Provider` property.
        pub fn provider_property() -> StyledProperty<Option<Rc<dyn ICursorProvider>>> {
            FerroProperty::register::<SetCursorFromProviderAction, _>("Provider", None)
        }
    }
}

impl StyledElementActionImpl for SetCursorFromProviderAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| crate::part_c::sender_as::<InputElement>(sender));
        let (Some(control), Some(provider)) = (control, this.provider()) else {
            return Some(Rc::new(false));
        };

        let cursor = provider.create_cursor();
        control.set_current_value(InputElement::cursor_property(), Some(cursor));
        Some(Rc::new(true))
    }
}

impl SetCursorFromProviderAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target control. This is a property of the object model; markup resolves a
    /// name given for it to the named control.
    pub fn target_control(&self) -> Option<Ref<InputElement>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<InputElement>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the [`ICursorProvider`]. This is a property of the object model.
    pub fn provider(&self) -> Option<Rc<dyn ICursorProvider>> {
        self.get_value(Self::provider_property())
    }

    pub fn set_provider(&self, value: Option<Rc<dyn ICursorProvider>>) {
        self.set_value(Self::provider_property(), value)
    }
}
