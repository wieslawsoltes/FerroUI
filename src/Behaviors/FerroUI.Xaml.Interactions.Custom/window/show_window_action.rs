//! Port of `Window/ShowWindowAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, Window};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Shows the specified window when executed.
#[repr(C)]
pub struct ShowWindowAction {
    base: StyledElementAction,
}

ferro_class!(ShowWindowAction: StyledElementAction);
ferro_class_info!(ShowWindowAction {
    new: ShowWindowAction::new,
    markup: {
        content: Window,
    },
});
ferroui_base::ferro_impl_classes!(ShowWindowAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ShowWindowAction {
        /// Identifies the `Window` property.
        pub fn window_property() -> StyledProperty<Option<Ref<Window>>> {
            FerroProperty::register::<ShowWindowAction, _>("Window", None)
        }
    }
}

impl StyledElementActionImpl for ShowWindowAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let Some(window) = this.window() else {
            return Some(Rc::new(false));
        };

        if let Some(control) = crate::part_c::sender_as::<Control>(sender) {
            window.set_data_context(control.data_context());
        }

        window.show();
        Some(Rc::new(true))
    }
}

impl ShowWindowAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the window instance to show. This is a property of the object model, and the
    /// content of the action in markup.
    pub fn window(&self) -> Option<Ref<Window>> {
        self.get_value(Self::window_property())
    }

    pub fn set_window(&self, value: Option<Ref<Window>>) {
        self.set_value(Self::window_property(), value)
    }
}
