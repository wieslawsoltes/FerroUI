//! Port of `Automation/Actions/ScreenReaderAnnounceAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that requests a screen reader announcement.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference looks up a method
/// `RequestScreenReaderAnnouncement` on the top level by reflection and returns `false` when the
/// top level has none. The top level of the framework has no such request (nor has the framework
/// the reference is built on), so the action finds the top level of the sender and returns
/// `false`, as the reference does without the method.
#[repr(C)]
pub struct ScreenReaderAnnounceAction {
    base: StyledElementAction,
}

ferro_class!(ScreenReaderAnnounceAction: StyledElementAction);
ferro_class_info!(ScreenReaderAnnounceAction { new: ScreenReaderAnnounceAction::new });
ferroui_base::ferro_impl_classes!(ScreenReaderAnnounceAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ScreenReaderAnnounceAction {
        /// Identifies the `Message` property.
        pub fn message_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ScreenReaderAnnounceAction, _>("Message", None)
        }
    }
}

impl StyledElementActionImpl for ScreenReaderAnnounceAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let control = sender.and_then(object_of).and_then(|sender| sender.cast::<Control>());
        let has_message = this.message().is_some_and(|message| !message.is_empty());
        if let (Some(control), true) = (control, has_message) {
            let visual: &Visual = &control;
            if let Some(_top_level) = TopLevel::get_top_level(Some(visual)) {
                // Deviation (DEVIATIONS.md, Behaviors): the top level has no request of a screen
                // reader announcement (the method the reference looks up is not found).
            }
        }

        Some(Rc::new(false))
    }
}

impl ScreenReaderAnnounceAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the message to announce.
    pub fn message(&self) -> Option<String> {
        self.get_value(Self::message_property())
    }

    pub fn set_message(&self, value: Option<String>) {
        self.set_value(Self::message_property(), value)
    }
}
