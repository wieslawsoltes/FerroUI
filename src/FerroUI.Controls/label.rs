use crate::primitives::TemplatedControlImpl;
use crate::{ContentControl, ContentControlImpl, ControlImpl};
use ferroui_base::input::{
    AccessKeyHandler, AccessKeyPressedEventArgs, InputElement, InputElementImpl, InputElementImplExt,
    PointerPressedEventArgs, PointerUpdateKind,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Nullable, Ref, StyledElementImpl, StyledProperty, Upcast, VisualImpl,
};

/// Label control. Focuses its `Target` when the label is activated: when it
/// is clicked, or when its access key is pressed.
#[repr(C)]
pub struct Label {
    base: ContentControl,
}

ferro_class!(Label: ContentControl);
ferroui_base::ferro_class_info!(Label { new: Label::new });
ferro_impl_classes!(
    Label: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ControlImpl for Label {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::LabelAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for Label {}

impl InputElementImpl for Label {
    fn on_access_key(this: &Self, e: &dyn IRoutedEventArgs) {
        this.label_activated(e);
    }

    /// Handles a pointer pressed event.
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        if e.get_current_point(Some(this.upcast())).properties.pointer_update_kind
            == PointerUpdateKind::LeftButtonPressed
        {
            this.label_activated(e);
        }
        Self::parent_on_pointer_pressed(this, e);
    }
}

ferroui_base::ferro_properties! { impl Label {
    ferro_property!(
        /// Defines the `Target` property.
        ///
        /// The label does not own its target, so the value is an element
        /// reference.
        pub fn target_property() -> StyledProperty<Option<ferroui_base::ElementRef<InputElement>>> {
            ferroui_base::data::core::ValueTypes::register_element_ref::<InputElement>();
            FerroProperty::register::<Label, _>("Target", None)
        }
    );
} }

impl Label {
    fn static_constructor() {
        AccessKeyHandler::access_key_pressed_event().add_class_handler::<Label>(Self::on_access_key_pressed);
        // The tab stop property defaults to false for labels.
        InputElement::is_tab_stop_property().override_default_value::<Label>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct() }
    }

    /// Initializes a new label.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the label's target.
    pub fn target(&self) -> Option<Ref<InputElement>> {
        ferroui_base::ElementRef::resolve(&self.get_value(Self::target_property()))
    }

    /// Sets the label's target.
    pub fn set_target(&self, value: impl Into<Nullable<InputElement>>) {
        self.set_value(Self::target_property(), ferroui_base::ElementRef::from_nullable(value.into().0))
    }

    fn label_activated(&self, e: &RoutedEventArgs) {
        let target = self.target();
        if let Some(target) = &target {
            target.focus();
        }
        e.set_handled(target.is_some());
    }

    fn on_access_key_pressed(label: &Label, e: &AccessKeyPressedEventArgs) {
        if e.handled() || e.target().is_some() {
            return;
        }

        e.set_target(label.target());
        e.set_handled(true);
    }
}
