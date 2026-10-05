use crate::primitives::{TemplatedControlImpl, ToggleButton, ToggleButtonImpl};
use crate::radio_button_group_manager::{register_radio_button, IRadioButton, RadioButtonGroupManager};
use crate::{ButtonImpl, ContentControlImpl, ControlImpl, MenuItemToggleType};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::rendering::IPresentationSource;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement, StyledElementImpl, StyledProperty,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents a button that allows a user to select a single option from a
/// group of options.
#[repr(C)]
pub struct RadioButton {
    base: ToggleButton,
    group_manager: RefCell<Option<Rc<RadioButtonGroupManager>>>,
}

ferro_class!(RadioButton: ToggleButton);
ferroui_base::ferro_class_info!(RadioButton { new: RadioButton::new });
ferro_impl_classes!(
    RadioButton: StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl
);

impl ControlImpl for RadioButton {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::RadioButtonAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for RadioButton {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == ToggleButton::is_checked_property().as_property() {
            this.is_checked_value_changed(change.get_new_value::<Option<bool>>());
        } else if change.property() == Self::group_name_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<String>>();
            this.on_group_name_changed(old_value.as_deref(), new_value.as_deref());
        }
    }
}

impl VisualImpl for RadioButton {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        let group_manager = this.group_manager.borrow().clone();
        if let Some(group_manager) = group_manager {
            group_manager.remove(&this.handle(), this.group_name().as_deref());
        }
        this.ensure_radio_group_manager(Some(e.presentation_source().clone()));
        Self::parent_on_attached_to_visual_tree(this, e);
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        let group_manager = this.group_manager.borrow_mut().take();
        if let Some(group_manager) = group_manager {
            group_manager.remove(&this.handle(), this.group_name().as_deref());
        }
    }
}

impl ToggleButtonImpl for RadioButton {
    fn toggle(this: &Self) {
        if !this.is_checked().unwrap_or_default() {
            this.set_current_value(ToggleButton::is_checked_property(), Some(true));
        }
    }
}

/// The radio button viewed as a member of a radio group.
struct RadioButtonHandle(Ref<RadioButton>);

impl IRadioButton for RadioButtonHandle {
    fn logical(&self) -> Ref<StyledElement> {
        self.0.clone().upcast()
    }

    fn group_name(&self) -> Option<String> {
        self.0.group_name()
    }

    fn toggle_type(&self) -> MenuItemToggleType {
        MenuItemToggleType::Radio
    }

    fn is_checked(&self) -> bool {
        self.0.is_checked().unwrap_or_default()
    }

    fn set_is_checked(&self, value: bool) {
        self.0.set_current_value(ToggleButton::is_checked_property(), Some(value));
    }
}

ferroui_base::ferro_properties! { impl RadioButton {
    ferro_property!(
        /// Defines the `GroupName` property.
        pub fn group_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<RadioButton, _>("GroupName", None)
        }
    );
} }

impl RadioButton {
    fn static_constructor() {
        register_radio_button::<RadioButton>(|radio_button| Rc::new(RadioButtonHandle(radio_button)));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ToggleButton::construct(), group_manager: RefCell::new(None) }
    }

    /// Creates a radio button.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The name that specifies which radio button controls are mutually
    /// exclusive.
    pub fn group_name(&self) -> Option<String> {
        self.get_value(Self::group_name_property())
    }

    pub fn set_group_name(&self, value: Option<String>) {
        self.set_value(Self::group_name_property(), value)
    }

    fn handle(&self) -> RadioButtonHandle {
        RadioButtonHandle(self.to_ref())
    }

    fn on_group_name_changed(&self, old_group_name: Option<&str>, new_group_name: Option<&str>) {
        if old_group_name.is_some_and(|name| !name.is_empty()) {
            let group_manager = self.group_manager.borrow().clone();
            if let Some(group_manager) = group_manager {
                group_manager.remove(&self.handle(), old_group_name);
            }
        }
        if new_group_name.is_some_and(|name| !name.is_empty()) {
            self.ensure_radio_group_manager(None);
        }
    }

    fn is_checked_value_changed(&self, value: Option<bool>) {
        if value.unwrap_or_default() {
            let group_manager = self.ensure_radio_group_manager(None);
            group_manager.on_checked_changed(&self.handle());
        }
    }

    fn ensure_radio_group_manager(&self, source: Option<Rc<dyn IPresentationSource>>) -> Rc<RadioButtonGroupManager> {
        let source = source.or_else(|| self.presentation_source());
        let group_manager = RadioButtonGroupManager::get_or_create_for_root(source.as_ref());
        *self.group_manager.borrow_mut() = Some(group_manager.clone());
        group_manager.add(&self.handle());
        group_manager
    }
}
