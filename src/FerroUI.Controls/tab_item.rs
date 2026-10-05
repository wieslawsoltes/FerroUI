use crate::i_selectable::{register_selectable, ISelectable};
use crate::metadata::PseudoClassesAttribute;
use crate::mixins::{PressedMixin, SelectableMixin};
use crate::primitives::{HeaderedContentControl, SelectingItemsControl, TemplatedControl, TemplatedControlImpl};
use crate::templates::IDataTemplate;
use crate::{ContentControlImpl, Control, ControlImpl, Dock, IconElement};
use ferroui_base::input::{
    AccessKeyHandler, AccessKeyPressedEventArgs, FocusChangedEventArgs, InputElement, InputElementImpl,
    InputElementImplExt, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, DirectProperty, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElement, StyledElementImpl,
    StyledProperty, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

/// An item in a `TabControl`.
#[repr(C)]
pub struct TabItem {
    base: HeaderedContentControl,
    tab_strip_placement: Cell<Option<Dock>>,
}

ferro_class!(TabItem: HeaderedContentControl);
ferroui_base::ferro_class_info!(TabItem { new: TabItem::new });
ferro_impl_classes!(
    TabItem: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ControlImpl for TabItem {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ListItemAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for TabItem {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == TemplatedControl::foreground_property().as_property()
            || change.property() == Self::icon_property().as_property()
        {
            this.update_icon_foreground();
        }
    }
}

impl InputElementImpl for TabItem {
    fn on_access_key(this: &Self, e: &dyn IRoutedEventArgs) {
        this.focus();
        this.set_current_value(Self::is_selected_property(), true);
        e.as_routed_event_args().set_handled(true);
    }

    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        this.update_selection_from_event(e);
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        this.update_selection_from_event(e);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        this.update_selection_from_event(e);
    }
}

impl ISelectable for TabItem {
    fn is_selected(&self) -> bool {
        TabItem::is_selected(self)
    }

    fn set_is_selected(&self, value: bool) {
        TabItem::set_is_selected(self, value)
    }
}

impl TabItem {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[":pressed", ":selected"]);
}

ferroui_base::ferro_properties! { impl TabItem {
    ferro_property!(
        /// Defines the `TabStripPlacement` property.
        pub fn tab_strip_placement_property() -> DirectProperty<TabItem, Option<Dock>> {
            FerroProperty::register_direct::<TabItem, _>("TabStripPlacement", |o| o.tab_strip_placement(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `IsSelected` property.
        pub fn is_selected_property() -> StyledProperty<bool> {
            SelectingItemsControl::is_selected_property().add_owner::<TabItem>()
        }
    );

    ferro_property!(
        /// Defines the `Icon` property.
        pub fn icon_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TabItem, _>("Icon", None)
        }
    );

    ferro_property!(
        /// Defines the `IconTemplate` property.
        pub fn icon_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TabItem, _>("IconTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `IndicatorTemplate` property.
        pub fn indicator_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TabItem, _>("IndicatorTemplate", None)
        }
    );
} }

impl TabItem {
    pub(crate) fn static_constructor() {
        register_selectable::<TabItem>();
        SelectableMixin::attach::<TabItem>(Self::is_selected_property());
        PressedMixin::attach::<TabItem>();
        InputElement::focusable_property().override_default_value::<TabItem>(true);
        StyledElement::data_context_property().changed().add_class_handler::<TabItem>(|x, e| x.update_header(e));
        crate::automation::AutomationProperties::control_type_override_property()
            .override_default_value::<TabItem>(Some(crate::automation::peers::AutomationControlType::TabItem));
        crate::automation::AutomationProperties::is_offscreen_behavior_property()
            .override_default_value::<TabItem>(crate::automation::IsOffscreenBehavior::FromClip);
        AccessKeyHandler::access_key_pressed_event().add_class_handler::<TabItem>(Self::on_access_key_pressed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: HeaderedContentControl::construct(), tab_strip_placement: Cell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the placement of this tab relative to the outer `TabControl`,
    /// if there is one.
    pub fn tab_strip_placement(&self) -> Option<Dock> {
        self.tab_strip_placement.get()
    }

    pub(crate) fn set_tab_strip_placement(&self, value: Option<Dock>) {
        self.set_and_raise_cell(Self::tab_strip_placement_property(), &self.tab_strip_placement, value);
    }

    /// Gets or sets the selection state of the item.
    pub fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }

    pub fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value)
    }

    /// Gets or sets the icon displayed alongside the tab header.
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }

    pub fn set_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), value)
    }

    /// Gets or sets the data template used to display the icon of the
    /// control.
    pub fn icon_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::icon_template_property())
    }

    pub fn set_icon_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::icon_template_property(), value)
    }

    /// Gets or sets the data template used to render the selection
    /// indicator.
    pub fn indicator_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::indicator_template_property())
    }

    pub fn set_indicator_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::indicator_template_property(), value)
    }

    fn on_access_key_pressed(tab_item: &TabItem, e: &AccessKeyPressedEventArgs) {
        if e.handled() || (e.target().is_some() && tab_item.is_selected()) {
            return;
        }

        e.set_target(Some(tab_item.to_ref().upcast()));
        e.set_handled(true);
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

    fn update_icon_foreground(&self) {
        let icon = self.icon().as_ref().and_then(Control::from_boxed).and_then(|icon| icon.cast::<IconElement>());
        if let Some(icon) = icon {
            match self.foreground() {
                Some(fg) => icon.set_value(TemplatedControl::foreground_property(), Some(fg)),
                None => icon.clear_value(TemplatedControl::foreground_property()),
            }
        }
    }

    fn update_header(&self, obj: &FerroPropertyChangedEventArgs<'_>) {
        let (old_value, new_value) = obj.get_old_and_new_value::<Option<BoxedValue>>();
        let header = self.header();

        if header.is_none() {
            let new_control = new_value.as_ref().and_then(Control::from_boxed);

            // A data context with a header: a headered content control.
            if let Some(headered) = new_control.as_ref().and_then(|c| crate::i_headered::as_headered(c)) {
                let headered_header = headered.header();
                if !crate::reference_equals(&header, &headered_header) {
                    self.set_current_value(HeaderedContentControl::header_property(), headered_header);
                }
            } else if new_control.is_none() {
                self.set_current_value(HeaderedContentControl::header_property(), new_value);
            }
        } else if crate::reference_equals(&header, &old_value) {
            self.set_current_value(HeaderedContentControl::header_property(), new_value);
        }
    }
}

