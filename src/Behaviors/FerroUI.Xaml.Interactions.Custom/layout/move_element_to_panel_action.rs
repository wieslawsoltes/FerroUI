//! Port of `Layout/MoveElementToPanelAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, Panel};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Moves the associated or target element to a specified [`Panel`].
#[repr(C)]
pub struct MoveElementToPanelAction {
    base: StyledElementAction,
}

ferro_class!(MoveElementToPanelAction: StyledElementAction);
ferro_class_info!(MoveElementToPanelAction {
    new: MoveElementToPanelAction::new,
    markup: {
        property_attributes: [
            TargetPanel: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(MoveElementToPanelAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl MoveElementToPanelAction {
        /// Identifies the `TargetPanel` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the panel, so the value
        /// is an element reference.
        pub fn target_panel_property() -> StyledProperty<Option<ElementRef<Panel>>> {
            ValueTypes::register_element_ref::<Panel>();
            FerroProperty::register::<MoveElementToPanelAction, _>("TargetPanel", None)
        }
    }
}

impl StyledElementActionImpl for MoveElementToPanelAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let sender = sender.and_then(object_of);
        let Some(element) = sender.clone().and_then(|sender| sender.cast::<Control>()) else {
            return Some(Rc::new(false));
        };

        let Some(target) = this.target_panel().or_else(|| sender.and_then(|sender| sender.cast::<Panel>())) else {
            return Some(Rc::new(false));
        };

        if let Some(source) = element.parent().and_then(|parent| parent.cast::<Panel>()) {
            source.children().remove(&element);
        }

        target.children().add(&element);
        Some(Rc::new(true))
    }
}

impl MoveElementToPanelAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the panel to move the element into. If not set, the sender is used as
    /// target. Markup resolves a name given for it to the named element.
    pub fn target_panel(&self) -> Option<Ref<Panel>> {
        ElementRef::resolve(&self.get_value(Self::target_panel_property()))
    }

    pub fn set_target_panel(&self, value: impl Into<Nullable<Panel>>) {
        self.set_value(Self::target_panel_property(), ElementRef::from_nullable(value.into().0))
    }
}
