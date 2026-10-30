//! Port of `Cursor/SetCursorAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{Cursor, InputElement};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Sets the cursor on a target control.
#[repr(C)]
pub struct SetCursorAction {
    base: StyledElementAction,
}

ferro_class!(SetCursorAction: StyledElementAction);
ferro_class_info!(SetCursorAction {
    new: SetCursorAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SetCursorAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetCursorAction {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the control, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<InputElement>>> {
            ValueTypes::register_element_ref::<InputElement>();
            FerroProperty::register::<SetCursorAction, _>("TargetControl", None)
        }

        /// Identifies the `Cursor` property.
        pub fn cursor_property() -> StyledProperty<Option<Rc<Cursor>>> {
            FerroProperty::register::<SetCursorAction, _>("Cursor", None)
        }
    }
}

impl StyledElementActionImpl for SetCursorAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| crate::part_c::sender_as::<InputElement>(sender));
        let (Some(control), Some(cursor)) = (control, this.cursor()) else {
            return Some(Rc::new(false));
        };

        control.set_current_value(InputElement::cursor_property(), Some(cursor));
        Some(Rc::new(true))
    }
}

impl SetCursorAction {
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

    /// Gets or sets the cursor to apply.
    pub fn cursor(&self) -> Option<Rc<Cursor>> {
        self.get_value(Self::cursor_property())
    }

    pub fn set_cursor(&self, value: Option<Rc<Cursor>>) {
        self.set_value(Self::cursor_property(), value)
    }
}
