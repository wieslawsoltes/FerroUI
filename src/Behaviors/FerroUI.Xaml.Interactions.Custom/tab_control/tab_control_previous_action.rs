//! Port of `TabControl/TabControlPreviousAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::TabControl;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Moves the target [`TabControl`] to the previous tab.
#[repr(C)]
pub struct TabControlPreviousAction {
    base: StyledElementAction,
}

ferro_class!(TabControlPreviousAction: StyledElementAction);
ferro_class_info!(TabControlPreviousAction {
    new: TabControlPreviousAction::new,
    markup: {
        property_attributes: [
            TabControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(TabControlPreviousAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl TabControlPreviousAction {
        /// Identifies the `TabControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the tab control, so the
        /// value is an element reference.
        pub fn tab_control_property() -> StyledProperty<Option<ElementRef<TabControl>>> {
            ValueTypes::register_element_ref::<TabControl>();
            FerroProperty::register::<TabControlPreviousAction, _>("TabControl", None)
        }
    }
}

impl StyledElementActionImpl for TabControlPreviousAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let tab_control =
            this.tab_control().or_else(|| sender.and_then(object_of).and_then(|sender| sender.cast::<TabControl>()));
        let Some(tab_control) = tab_control else {
            return Some(Rc::new(false));
        };

        if tab_control.selected_index() > 0 {
            tab_control.set_selected_index(tab_control.selected_index() - 1);
        }

        None
    }
}

impl TabControlPreviousAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the tab control instance this action will operate on. This is a property of
    /// the object model. Markup resolves a name given for it to the named element.
    pub fn tab_control(&self) -> Option<Ref<TabControl>> {
        ElementRef::resolve(&self.get_value(Self::tab_control_property()))
    }

    pub fn set_tab_control(&self, value: impl Into<Nullable<TabControl>>) {
        self.set_value(Self::tab_control_property(), ElementRef::from_nullable(value.into().0))
    }
}
