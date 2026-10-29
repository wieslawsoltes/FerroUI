//! Port of `Actions/PopupAction.cs`.

use super::control_of;
use ferroui_base::data::BindingPriority;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElement, StyledElementImpl, StyledProperty,
};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{Control, PlacementMode};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// Opens a popup at the pointer with [`child`](Self::child) as its content (the content of the
/// action in markup). The popup is created on the first execution, as a logical child of the
/// sender, and follows the data context of the sender.
#[repr(C)]
pub struct PopupAction {
    base: StyledElementAction,
    popup: RefCell<Option<Ref<Popup>>>,
}

ferro_class!(PopupAction: StyledElementAction);
ferro_class_info!(PopupAction {
    new: PopupAction::new,
    markup: {
        content: Child,
    },
});
ferroui_base::ferro_impl_classes!(PopupAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl PopupAction {
        /// Identifies the `Child` property.
        pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<PopupAction, _>("Child", None)
        }
    }
}

impl StyledElementActionImpl for PopupAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        if this.popup.borrow().is_none() {
            let parent = control_of(sender);

            let popup = Popup::new();
            popup.set_placement(PlacementMode::Pointer);
            popup.set_placement_target(parent.clone());
            popup.set_is_light_dismiss_enabled(true);

            if let Some(control) = &parent {
                Self::bind_to_data_context(control, &popup);
            }

            popup.set_parent(parent.map(|parent| parent.upcast::<StyledElement>()));
            *this.popup.borrow_mut() = Some(popup);
        }

        let popup = this.popup.borrow().clone();
        if let Some(popup) = popup {
            popup.set_child(this.child());
            popup.open();
        }

        None
    }
}

impl PopupAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct(), popup: RefCell::new(None) }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the content of the popup. This is a property of the object model.
    pub fn child(&self) -> Option<Ref<Control>> {
        self.get_value(Self::child_property())
    }

    pub fn set_child(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::child_property(), value.into().0)
    }

    fn bind_to_data_context(source: &Control, target: &Control) {
        let source_object: &FerroObject = source;
        let data = FerroObjectExtensions::get_observable(source_object, StyledElement::data_context_property());
        let _ = target.bind(StyledElement::data_context_property(), data, BindingPriority::LocalValue);
    }
}
