use crate::i_selectable::{register_selectable, ISelectable};
use crate::metadata::PseudoClassesAttribute;
use crate::mixins::{PressedMixin, SelectableMixin};
use crate::primitives::{SelectingItemsControl, TemplatedControlImpl};
use crate::{ContentControl, ContentControlImpl, ControlImpl};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, KeyEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, StyledProperty, VisualImpl,
};

/// A selectable item in a `ListBox`.
#[repr(C)]
pub struct ListBoxItem {
    base: ContentControl,
}

ferro_class!(ListBoxItem: ContentControl);
ferroui_base::ferro_class_info!(ListBoxItem { new: ListBoxItem::new });
ferro_impl_classes!(
    ListBoxItem: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for ListBoxItem {}

impl InputElementImpl for ListBoxItem {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        this.update_selection_from_event(e);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        this.update_selection_from_event(e);
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        this.update_selection_from_event(e);
    }
}

impl ControlImpl for ListBoxItem {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ListItemAutomationPeer::new(this).upcast()
    }
}

impl ISelectable for ListBoxItem {
    fn is_selected(&self) -> bool {
        ListBoxItem::is_selected(self)
    }

    fn set_is_selected(&self, value: bool) {
        ListBoxItem::set_is_selected(self, value)
    }
}

impl ListBoxItem {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[":pressed", ":selected"]);
}

ferroui_base::ferro_properties! { impl ListBoxItem {
    ferro_property!(
        /// Defines the `IsSelected` property.
        pub fn is_selected_property() -> StyledProperty<bool> {
            SelectingItemsControl::is_selected_property().add_owner::<ListBoxItem>()
        }
    );
} }

impl ListBoxItem {
    pub(crate) fn static_constructor() {
        register_selectable::<ListBoxItem>();
        SelectableMixin::attach::<ListBoxItem>(Self::is_selected_property());
        PressedMixin::attach::<ListBoxItem>();
        InputElement::focusable_property().override_default_value::<ListBoxItem>(true);
        crate::automation::AutomationProperties::is_offscreen_behavior_property()
            .override_default_value::<ListBoxItem>(crate::automation::IsOffscreenBehavior::FromClip);
        crate::platform::PlatformFeedback::feedback_type_property()
            .override_default_value::<ListBoxItem>(crate::platform::FeedbackType::Auto);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the selection state of the item.
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }

    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value)
    }

    /// Updates the selection of the owning selecting items control from an
    /// event raised on this container.
    pub fn update_selection_from_event(&self, e: &dyn IRoutedEventArgs) -> bool {
        let this = self.to_ref().upcast();
        match SelectingItemsControl::selecting_items_control_from_item_container(&this) {
            Some(owner) => owner.update_selection_from_event(&this, e),
            None => false,
        }
    }
}
