//! Port of `Window/CloseWindowAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::{TopLevel, Window};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Closes the associated or target window when executed.
#[repr(C)]
pub struct CloseWindowAction {
    base: StyledElementAction,
}

ferro_class!(CloseWindowAction: StyledElementAction);
ferro_class_info!(CloseWindowAction {
    new: CloseWindowAction::new,
    markup: {
        property_attributes: [
            TargetWindow: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(CloseWindowAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl CloseWindowAction {
        /// Identifies the `TargetWindow` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the window, so the value
        /// is an element reference.
        pub fn target_window_property() -> StyledProperty<Option<ElementRef<Window>>> {
            ValueTypes::register_element_ref::<Window>();
            FerroProperty::register::<CloseWindowAction, _>("TargetWindow", None)
        }
    }
}

impl StyledElementActionImpl for CloseWindowAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let window = this
            .target_window()
            .or_else(|| crate::part_c::sender_as::<Window>(sender))
            .or_else(|| {
                let visual = crate::part_c::sender_as::<Visual>(sender);
                TopLevel::get_top_level(visual.as_deref()).and_then(|top_level| top_level.cast::<Window>())
            });
        let Some(window) = window else {
            return Some(Rc::new(false));
        };

        window.close();
        Some(Rc::new(true))
    }
}

impl CloseWindowAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target window. This is a property of the object model; markup resolves a
    /// name given for it to the named window.
    pub fn target_window(&self) -> Option<Ref<Window>> {
        ElementRef::resolve(&self.get_value(Self::target_window_property()))
    }

    pub fn set_target_window(&self, value: impl Into<Nullable<Window>>) {
        self.set_value(Self::target_window_property(), ElementRef::from_nullable(value.into().0))
    }
}
